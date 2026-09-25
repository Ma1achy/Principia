"""Probe scheduler: subprocess isolation, hard timeout, incremental JSON.

Probes cost seconds normally but can hang (see xp_probe docstring), and long
runs have been lost to timeouts -- so every completed probe is flushed to disk
immediately.
"""
import json, os, subprocess, sys, tempfile, time

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = os.path.join(HERE, 'xp_results')


class Store:
    """Append-only JSON store, rewritten after every probe."""

    def __init__(self, path):
        self.path = os.path.join(RESULTS, path)
        self.recs = []
        if os.path.exists(self.path):
            with open(self.path) as f:
                self.recs = json.load(f)

    def key(self, a):
        return json.dumps({k: a[k] for k in sorted(a) if not k.startswith('_')},
                          sort_keys=True)

    def has(self, a):
        return any(r.get('_key') == self.key(a) for r in self.recs)

    def get(self, a):
        k = self.key(a)
        return next((r for r in self.recs if r.get('_key') == k), None)

    def add(self, rec):
        self.recs.append(rec)
        with open(self.path, 'w') as f:
            json.dump(self.recs, f)


def probe(store, timeout=180, **kw):
    """Run one probe unless already stored. Returns the record, or a stub with
    _failed set. Never raises."""
    if store.has(kw):
        return store.get(kw)
    fd, tmp = tempfile.mkstemp(suffix='.json', dir=RESULTS)
    os.close(fd)
    payload = dict(kw, _out=tmp)
    t0 = time.time()
    status, rec = 'ok', None
    try:
        subprocess.run([sys.executable, os.path.join(HERE, 'xp_probe.py'),
                        json.dumps(payload)],
                       cwd=HERE, timeout=timeout, check=True,
                       capture_output=True)
        with open(tmp) as f:
            rec = json.load(f)
    except subprocess.TimeoutExpired:
        status = 'timeout'
    except subprocess.CalledProcessError as e:
        status = 'error: ' + e.stderr.decode()[-300:]
    finally:
        if os.path.exists(tmp):
            os.unlink(tmp)
    el = time.time() - t0
    if rec is None:
        rec = dict(kw, _failed=status)
    rec['_key'] = store.key(kw)
    rec['_seconds'] = round(el, 2)
    rec['_status'] = status
    store.add(rec)
    return rec
