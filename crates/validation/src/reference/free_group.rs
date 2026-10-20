//! A reference free-group reducer, for the word buffer's property tests (dd_simstate_payload §3; dd_generation_root
//! §3.3). It shares no code with the kernel's append: free reduction is a stack of symbols, the word's integer `W` is
//! payload §3's closed form summed in u128, and the continuation table is transcribed here from payload §3.
//!
//! Symbol codes `a = 0, A = 1, b = 2, B = 3` (payload §3). The truncation rule is payload §3's length cap: a
//! non-cancelling symbol arriving at [`CAPACITY`] symbols truncates the word, after which every symbol is ignored; a
//! cancelling one at the cap still pops. A truncated word keeps its 76 symbols, a prefix, not the reduced word.

/// The capacity, 76 symbols in 121 bits (payload §3).
pub const CAPACITY: usize = 76;

/// The truncated word's `length` (payload §3).
pub const TRUNCATED: u32 = 127;

/// `inverse[s]` (payload §3).
pub const INVERSE: [u32; 4] = [1, 0, 3, 2];

/// `cont_symbol[e][prev]`, the symbol digit `e` continues `prev` with: payload §3's frozen table.
pub const CONT_SYMBOL: [[u32; 4]; 3] = [[0, 1, 2, 3], [2, 3, 0, 1], [3, 2, 1, 0]];

/// A stream freely reduced: the reduced symbols (the retained prefix, once truncated), and whether the cap truncated
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reduced {
    pub symbols: Vec<u32>,
    pub truncated: bool,
}

impl Reduced {
    /// The word's `length` field: the symbol count, or [`TRUNCATED`].
    pub fn length(&self) -> u32 {
        if self.truncated {
            TRUNCATED
        } else {
            self.symbols.len() as u32
        }
    }
}

/// `stream` (symbol codes, each < 4) freely reduced by a stack: a symbol after its inverse cancels both; a push onto
/// [`CAPACITY`] symbols truncates the word, and the rest of the stream is ignored.
pub fn reduce(stream: &[u32]) -> Reduced {
    let mut symbols: Vec<u32> = Vec::with_capacity(CAPACITY);
    for &s in stream {
        assert!(s < 4, "{s} is not a symbol code");
        if symbols.last() == Some(&INVERSE[s as usize]) {
            symbols.pop();
        } else if symbols.len() == CAPACITY {
            return Reduced {
                symbols,
                truncated: true,
            };
        } else {
            symbols.push(s);
        }
    }
    Reduced {
        symbols,
        truncated: false,
    }
}

/// The digit `e` with `cont_symbol[e][prev] == next`; `None` where `next` is `prev`'s inverse, which no digit
/// continues.
pub fn digit(prev: u32, next: u32) -> Option<u32> {
    (0..3).find(|&e| CONT_SYMBOL[e as usize][prev as usize] == next)
}

/// `W` of a freely-reduced word by payload §3's closed form, `W = d₀·3^(ℓ−1) + Σ_{k=1}^{ℓ−1} e_k·3^(ℓ−1−k)`, summed
/// term by term in u128; 0 for the empty word. Panics on a word that is not freely reduced, or longer than
/// [`CAPACITY`].
pub fn closed_form(symbols: &[u32]) -> u128 {
    assert!(symbols.len() <= CAPACITY, "{} symbols", symbols.len());
    let Some(&d0) = symbols.first() else {
        return 0;
    };
    let l = symbols.len() as u32;
    let mut w = u128::from(d0) * 3u128.pow(l - 1);
    for (k, pair) in symbols.windows(2).enumerate() {
        let e = digit(pair[0], pair[1])
            .expect("a freely-reduced word never continues a symbol with its inverse");
        w += u128::from(e) * 3u128.pow(l - 2 - k as u32);
    }
    w
}

/// The word buffer's `vec4<u32>` for `reduced` (payload §3): `W`'s low 96 bits in `x`, `y`, `z`, its high bits in
/// `.w` bits 0–24, `length` in bits 25–31. Panics when `W` does not fit 121 bits.
pub fn packed(reduced: &Reduced) -> [u32; 4] {
    let w = closed_form(&reduced.symbols);
    assert!(w < 1 << 121, "W = {w} does not fit 121 bits");
    [
        w as u32,
        (w >> 32) as u32,
        (w >> 64) as u32,
        (w >> 96) as u32 | (reduced.length() << 25),
    ]
}
