//! The latent space's dimension (chart_decoder_contract Part 2: "Latent controls `z ∈ ℝ⁸`"), read from [`Latent`].

use crate::contract::sim_config::{Latent, LATENT_DIM};

/// `LATENT_DIM` is `Latent`'s length, eight, and the three blocks of Part 2 fill it: 2 + 4 + 2.
fn check_dimension(dim: usize) {
    assert_eq!(dim, 8, "z ∈ ℝ⁸ (chart_decoder_contract Part 2)");
    let z: Latent = [0.0; 8];
    assert_eq!(z.len(), dim, "Latent holds LATENT_DIM components");
    let (configuration, momentum, mass) = (2, 4, 2);
    assert_eq!(
        configuration + momentum + mass,
        dim,
        "the blocks z[0:2], z[2:6], z[6:8] fill it"
    );
}

#[test]
fn latent_dimension_is_eight() {
    check_dimension(LATENT_DIM);
}

validation::negative_control!(
    latent_dimension_is_eight,
    "a ten-dimensional latent, the old 10D space, must be rejected",
    expected = "z ∈ ℝ⁸",
    check_dimension(10)
);
