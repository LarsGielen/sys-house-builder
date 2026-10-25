use super::*;
use crate::input::{Action, Binding, Bindings, Context, ModifierRule, Trigger};

#[test]
fn default_bindings_describe_the_standard_controls() {
	let bindings = Bindings::default();
	assert_eq!(bindings.describe(Action::Save), "Ctrl+S");
	assert_eq!(bindings.describe(Action::Redo), "Ctrl+Shift+Z");
	assert_eq!(bindings.describe(Action::Cancel), "Esc");
	assert_eq!(bindings.describe(Action::SuppressSnapping), "Shift");
	assert_eq!(bindings.describe(Action::ToolPress), "Left click");
}

#[test]
fn templates_follow_a_rebinding() {
	let mut bindings = Bindings::default();
	assert_eq!(
		bindings.fill("{FieldSubmit} applies, {Cancel} cancels"),
		"Enter applies, Esc cancels"
	);
	bindings
		.0
		.retain(|binding| binding.action != Action::Cancel);
	bindings.0.push(Binding {
		trigger: Trigger::Key(KeyCode::KeyQ),
		control: ModifierRule::Ignored,
		shift: ModifierRule::Ignored,
		context: Context::Viewport,
		action: Action::Cancel,
	});
	assert_eq!(bindings.fill("{Cancel} cancels"), "Q cancels");
	bindings
		.0
		.retain(|binding| binding.action != Action::Cancel);
	assert_eq!(bindings.fill("{Cancel} cancels"), "unbound cancels");
}

#[test]
fn hint_text_nodes_are_filled_from_the_bindings() {
	let mut app = App::new();
	app.init_resource::<Bindings>()
		.add_systems(Update, fill_hints);
	let entity = app.world_mut().spawn(hint("Press {Delete} to remove")).id();
	app.update();
	assert_eq!(
		app.world().get::<Text>(entity).unwrap().0,
		"Press Del to remove"
	);
}
