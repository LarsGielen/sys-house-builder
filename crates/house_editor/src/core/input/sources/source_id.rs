/// Which kind of device an input comes from. Arbitration treats each source as one party, so
/// inputs of the same source combine and different sources compete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(pub(in crate::core::input) u32);

impl SourceId {
	pub const KEYBOARD: SourceId = SourceId(0);
	pub const MOUSE: SourceId = SourceId(1);
	pub const UI: SourceId = SourceId(2);
}
