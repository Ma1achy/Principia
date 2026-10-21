"""A data-driven, compositional post-process system.

A *medium* is a graph of small typed nodes, written as JSON. Nodes are registered with @node; a
graph names its nodes, wires their inputs to other nodes' outputs, and sets their parameters.

References (used for inputs, and for parameters that should vary per pixel):
    "src"            the image coming in (the render, or the output of an upstream medium)
    "data.<name>"    a measured field supplied with the render (lifetime, sensitivity, ...)
    "<node>"         a node's first output;  "<node>.<port>"  a named output
Parameter values:
    a literal        5, 0.8, "#efe9dc", [1, 2]
    "@<ref>"         bind the parameter to a field: it varies per pixel
    "$<name>"        a setting the preset exposes; callers can override it
A node whose op is "preset:<name>" runs another preset as a sub-graph, so media compose.
"""
import json, os, hashlib
import numpy as np

REGISTRY = {}

def node(name, inputs=(), outputs=('out',), **params):
    """Register a node. `params` are the defaults; inputs are positional data the node consumes."""
    def deco(fn):
        REGISTRY[name] = dict(fn=fn, inputs=tuple(inputs), outputs=tuple(outputs), params=params, doc=(fn.__doc__ or '').strip())
        return fn
    return deco

PRESET_DIR = os.path.join(os.path.dirname(__file__), 'presets')
def load_preset(name):
    with open(os.path.join(PRESET_DIR, name + '.json')) as f:
        return json.load(f)

class Context:
    def __init__(self, src, data=None, seed=0):
        self.src = src; self.data = data or {}; self.seed = seed
        self.H, self.W = src.shape[:2]
    def rng(self, key):
        h = int(hashlib.sha1(f'{self.seed}:{key}'.encode()).hexdigest()[:8], 16)
        return np.random.default_rng(h)

class Graph:
    def __init__(self, spec, ctx, settings=None, path=''):
        self.spec = spec; self.ctx = ctx; self.memo = {}; self.path = path
        self.settings = dict(spec.get('settings', {})); self.settings.update(settings or {})

    def ref(self, r):
        if '|' in r:                                     # "data.lifetime|proxy": the data if supplied, else the stand-in
            first, _, rest = r.partition('|')
            try: return self.ref(first)
            except KeyError: return self.ref(rest)
        if r == 'src': return self.ctx.src
        if r.startswith('data.'):
            k = r[5:]
            if k not in self.ctx.data: raise KeyError(f'no data field {k!r} was supplied')
            return self.ctx.data[k]
        nid, _, port = r.partition('.')
        out = self.eval(nid)
        if isinstance(out, dict): return out[port] if port else next(iter(out.values()))
        return out

    def param(self, v):
        if isinstance(v, str) and v.startswith('$'):
            name, op, k = v[1:].partition('+')
            if not op: name, op, k = v[1:].partition('-')
            base = self.param(self.settings[name])
            return base if not op else (base + int(k) if op == '+' else base - int(k))
        if isinstance(v, str) and v.startswith('@'):
            return self.ref(v[1:])
        if isinstance(v, list): return [self.param(x) for x in v]
        return v

    def eval(self, nid):
        if nid in self.memo: return self.memo[nid]
        n = self.spec['nodes'][nid]; op = n['op']
        ins = {k: (self.param(v) if v.startswith('$') else self.ref(v)) for k, v in n.get('in', {}).items()}
        ps = {k: self.param(v) for k, v in n.get('p', {}).items()}
        if op.startswith('preset:'):
            sub = load_preset(op[7:])
            src = ins.get('src', self.ctx.src)
            sctx = Context(src, self.ctx.data, self.ctx.seed); sctx.rng = (lambda key, base=self.ctx, pre=self.path+nid+'/': base.rng(pre+key))
            out = Graph(sub, sctx, ps, self.path+nid+'/').run()
        else:
            spec = REGISTRY[op]
            kw = dict(spec['params']); kw.update(ps)
            missing = [i for i in spec['inputs'] if i not in ins]
            if missing: raise ValueError(f'{self.path}{nid} ({op}) is missing inputs {missing}')
            out = spec['fn'](_NodeCtx(self.ctx, self.path+nid), **ins, **kw)
        self.memo[nid] = out
        return out

    def run(self):
        return self.ref(self.spec['out'])

class _NodeCtx:
    """What a node sees: the frame size, the data fields, and its own deterministic random stream."""
    def __init__(self, ctx, key): self.ctx = ctx; self.key = key; self.H, self.W = ctx.H, ctx.W
    def rng(self, salt=''): return self.ctx.rng(self.key + salt)
    @property
    def data(self): return self.ctx.data

def render(preset, src, data=None, seed=0, **settings):
    """Run a preset (a name, or a spec dict) on an RGB float image."""
    spec = load_preset(preset) if isinstance(preset, str) else preset
    settings.setdefault('seed', seed)
    return Graph(spec, Context(src, data, seed), settings).run()
