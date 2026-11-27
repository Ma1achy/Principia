// @version 2
// A read-side field as a source: a scalar in the field's x, a vector in its xyz. `as_field` is written into the code
// as the chosen field's own read, so the field is what the proof, the field set and the code key see.
// @name ctx.{field}
// @group Source
// @about A read-side field of the system, chosen by the Field setting; the out-port's kind follows it.
// @maps ctx.{field}
// @glyph dropdown
// @uniform field: field = state any
// @control field field "Field"
fn source(ctx: Ctx) -> Field { return as_field(uniforms.field); }
