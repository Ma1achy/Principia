//! What the editor reads to dim a control (`describe::standing`): a control the node's shader cannot read into its
//! result with the settings as they are is "not now", with the choice that would let it; one it never reads is
//! "never"; the rest matter. Its control arm feeds the proof the wrong setting.

use std::collections::BTreeMap;

use engine::contract::stain::describe::{standing, Matters, Needs, Standing};
use engine::contract::stain::graph::{NodeKind, Occupant};
use validation::negative_control;

/// A colour whose choice decides which of two colours it draws, and a gain it never reads.
const CHOICE: &str = r#"// @uniform mode: u32 = 0
// @control mode choice("warm", "cool") "Mode"
// @uniform warm: vec3<f32> = (1.0, 0.5, 0.0)
// @control warm colour "Warm"
// @uniform cool: vec3<f32> = (0.0, 0.5, 1.0)
// @control cool colour "Cool"
// @uniform gain: f32 = 1.0 [0, 2]
fn colour(ctx: Ctx) -> vec3<f32> {
    if (uniforms.mode == 1u) {
        return uniforms.cool;
    }
    return uniforms.warm;
}
"#;

fn at(mode: f64) -> Vec<Standing> {
    let params = BTreeMap::from([("mode".to_owned(), vec![mode])]);
    standing(NodeKind::Colour, &Occupant::Custom(CHOICE.into()), &params)
        .unwrap_or_else(|e| panic!("{e}"))
}

fn check(got: &[Standing], dimmed: &str, needs: f64, named: &str) {
    let of = |u: &str| {
        got.iter()
            .find(|s| s.uniform == u)
            .map(|s| s.matters.clone())
            .unwrap_or_else(|| panic!("no standing for `{u}`: {got:?}"))
    };
    assert_eq!(of("mode"), Matters::Now, "{got:?}");
    assert_eq!(of("gain"), Matters::Never, "{got:?}");
    let want = Matters::NotNow {
        needs: Some(Needs {
            uniform: "mode".into(),
            label: "Mode".into(),
            value: needs,
            named: named.into(),
        }),
    };
    assert_eq!(
        of(dimmed),
        want,
        "`{dimmed}` is not dimmed with the setting that would let it matter"
    );
    let other = if dimmed == "warm" { "cool" } else { "warm" };
    assert_eq!(of(other), Matters::Now, "{got:?}");
}

#[test]
fn standing_dims_with_the_setting() {
    // Pass criterion: at mode 0 (warm), `cool` is "not now" and needs mode = 1, "cool"; at mode 1, `warm` needs 0,
    // "warm"; `mode` always matters and `gain` never does.
    check(&at(0.0), "cool", 1.0, "cool");
    check(&at(1.0), "warm", 0.0, "warm");
    // The same code with the label edited is the same compiled shader; its standing reads the new label.
    let relabelled = CHOICE.replace("\"Mode\"", "\"Look\"");
    let params = BTreeMap::from([("mode".to_owned(), vec![0.0])]);
    let got = standing(NodeKind::Colour, &Occupant::Custom(relabelled), &params)
        .unwrap_or_else(|e| panic!("{e}"));
    let label = got.iter().find_map(|s| match &s.matters {
        Matters::NotNow { needs: Some(n) } => Some(n.label.clone()),
        _ => None,
    });
    assert_eq!(label.as_deref(), Some("Look"), "{got:?}");
    assert!(standing(NodeKind::Out, &Occupant::None, &BTreeMap::new())
        .unwrap_or_else(|e| panic!("{e}"))
        .is_empty());
}

negative_control!(
    standing_dims_with_the_setting,
    "the standing at mode 1 checked as mode 0's",
    expected = "is not dimmed with the setting",
    check(&at(1.0), "cool", 1.0, "cool")
);
