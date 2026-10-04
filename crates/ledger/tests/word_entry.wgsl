// The word accessors' test entry point (REQ-PAY-023, REQ-PAY-028, REQ-PAY-033): appended to the generated fragment
// unpack layer, `crates/render/frag/generated/payload_unpack.wgsl`, it reads each invocation's word through the
// generated accessors only (payload §3, §6). A selector is `case << 8 | what`: `case` the word's index in `t_words`,
// four u32s each (x y z w), and `what` 0 `fgw_length_raw`, 1 `fgw_truncated`, 2 `fgw_reduced_length_valid`,
// 3 `fgw_reduced_length`, 4 `fgw_retained_prefix_length`, 5 `sd_last_symbol_valid(fgw_length_raw(word))`, and from 6
// `fgw_symbol(word, what − 6)`; a bool as 0 or 1. Group 0 is the harness's: selectors, words, then the output.

@group(0) @binding(0) var<storage, read> t_sel: array<u32>;
@group(0) @binding(1) var<storage, read> t_words: array<u32>;
@group(0) @binding(2) var<storage, read_write> t_out: array<u32>;

fn t_word(c: u32) -> vec4<u32> {
    return vec4<u32>(t_words[4u * c], t_words[4u * c + 1u], t_words[4u * c + 2u], t_words[4u * c + 3u]);
}

fn t_read(word: vec4<u32>, what: u32) -> u32 {
    switch what {
        case 0u: { return fgw_length_raw(word); }
        case 1u: { return select(0u, 1u, fgw_truncated(word)); }
        case 2u: { return select(0u, 1u, fgw_reduced_length_valid(word)); }
        case 3u: { return fgw_reduced_length(word); }
        case 4u: { return fgw_retained_prefix_length(word); }
        case 5u: { return select(0u, 1u, sd_last_symbol_valid(fgw_length_raw(word))); }
        default: { return fgw_symbol(word, what - 6u); }
    }
}

@compute @workgroup_size(64)
fn t_word_read(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&t_sel)) {
        let s = t_sel[id.x];
        t_out[id.x] = t_read(t_word(s >> 8u), s & 0xffu);
    }
}
