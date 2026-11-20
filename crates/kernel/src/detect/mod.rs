//! The detectors, wrapper machinery and the `SimState`'s outcome producer (integrator_contract Part 7; dd_integrator
//! §3.6): shared by every occupant, they read the state whichever `STEP` produced it.
//!
//! - [`collision`]: the per-`STEP` collision test on the projected state, counting the pairs inside `r_coll` before
//!   classifying, and the `dmin_pair` latch, both reading the step's one computation of the pairs' squared separations,
//!   which the substep bucket reads too (REQ-EVT-003, REQ-EVT-004, REQ-PAY-047).
//! - [`t0`]: the dispatch-time pass on the decoded IC, before the first step (integrator_contract Part 5, "t=0
//!   terminals are real outcomes"; R-60): a pure function of the decode's output and the sim key's `r_coll`.
//! - [`escape`]: the escape test at the occupant's sampling points, the shape point settled over the window and a body
//!   unbound, with the lagged shape register, the escaper and the ionisation gate (Part 7; pitfalls §2).

pub mod collision;
pub mod escape;
pub mod t0;
