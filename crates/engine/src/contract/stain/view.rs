//! The render type a stain is drawn with (`render::stain_view`), reached through the contract: the GUI's canvas draws
//! the field's records through it.

pub use render::pipeline_cache::{CompileError, FrameInputs, NodeKey};
pub use render::stain_view::{Placement, Records, Stain, StainView};
