mod wall_render;

use std::collections::HashMap;

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use wall_graph::{OpeningSpec, Wall, WallDimensions, WallGraph};
use wall_mesh::{MeshSettings, SurfaceKind, SurfaceSource};

use wall_render::{graph_to_world, tagged_bevy_meshes, wire_edges};

/// Centerlines are drawn this far above each wall's top so the walls do not hide them.
const CENTERLINE_LIFT: f32 = 0.05;

#[derive(Resource)]
struct DemoGraph(WallGraph);

#[derive(Resource)]
struct ShowCenterlines(bool);

#[derive(Resource, Default)]
struct ViewSettings {
	mode: ViewMode,
	wire_overlay: bool,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
	#[default]
	Shaded,
	ByWall,
	BySurface,
	Wireframe,
}

#[derive(Component)]
struct WallPiece {
	materials: [Handle<StandardMaterial>; 3],
}

#[derive(Resource)]
struct WireEdges(Vec<(Vec3, Vec3)>);

#[derive(Clone, Copy)]
enum InspectionArea {
	House,
	Arcs,
	Bevels,
	Corners,
	Overview,
}

impl InspectionArea {
	fn camera(self) -> (Vec3, f32) {
		match self {
			Self::House => (Vec3::new(4.0, 1.0, 0.0), 18.0),
			Self::Arcs => (Vec3::new(-6.0, 1.0, -8.0), 12.0),
			Self::Bevels => (Vec3::new(8.0, 1.0, -8.0), 15.0),
			Self::Corners => (Vec3::new(0.0, 1.0, -16.0), 14.0),
			Self::Overview => (Vec3::new(4.0, 1.0, -7.0), 36.0),
		}
	}
}

#[derive(Component, Clone, Copy)]
enum ToolbarAction {
	View(ViewMode),
	Focus(InspectionArea),
	ToggleEdges,
	ToggleCenterlines,
}

#[derive(Component)]
struct OrbitCamera {
	focus: Vec3,
	distance: f32,
	yaw: f32,
	pitch: f32,
}

fn build_demo_graph() -> WallGraph {
	let mut graph = WallGraph::new();

	let bottom_left = Vec2::new(-3.0, -2.0);
	let bottom_right = Vec2::new(3.0, -2.0);
	let top_right = Vec2::new(3.0, 2.0);
	let top_left = Vec2::new(-3.0, 2.0);
	let arc_end = Vec2::new(5.0, 0.0);

	let bottom = graph
		.add_wall(bottom_left, bottom_right)
		.expect("demo wall")[0];
	graph.add_wall(bottom_right, top_right).expect("demo wall");
	let top = graph.add_wall(top_right, top_left).expect("demo wall")[0];
	graph.add_wall(top_left, bottom_left).expect("demo wall");
	graph
		.add_arc(top_right, arc_end, -std::f32::consts::FRAC_PI_2)
		.expect("demo arc");
	graph
		.add_wall(Vec2::new(4.0, -0.6), Vec2::new(4.0, 2.6))
		.expect("demo crossing");

	graph
		.add_opening(
			bottom,
			OpeningSpec {
				center_distance: 3.0,
				width: 1.0,
				bottom: 0.0,
				height: 2.1,
			},
		)
		.expect("demo door opening");
	graph
		.add_opening(
			top,
			OpeningSpec {
				center_distance: 3.0,
				width: 1.2,
				bottom: 0.9,
				height: 1.2,
			},
		)
		.expect("demo window opening");
	let curved_wall = graph
		.walls()
		.find(|&wall| {
			(graph.wall_position(wall, 0.0) == Some(arc_end)
				|| graph.wall_position(wall, 1.0) == Some(arc_end))
				&& graph
					.sample_wall(wall, 0.01)
					.expect("valid demo wall")
					.len() > 2
		})
		.expect("arc piece ending at demo endpoint");
	graph
		.add_opening(
			curved_wall,
			OpeningSpec {
				center_distance: graph.wall_length(curved_wall).unwrap() * 0.5,
				width: 0.5,
				bottom: 0.8,
				height: 1.0,
			},
		)
		.expect("demo curved opening");

	add_junction_samples(&mut graph);
	add_visual_checks(&mut graph);
	graph
}

/// A second cluster whose junctions differ in thickness and height, with stacked windows.
fn add_junction_samples(graph: &mut WallGraph) {
	let through_start = Vec2::new(8.0, -2.0);
	let t_junction = Vec2::new(11.0, -2.0);
	let corner = Vec2::new(14.0, -2.0);

	graph.add_wall(through_start, corner).expect("demo wall");
	graph
		.add_wall_with_dimensions(
			t_junction,
			Vec2::new(11.0, 1.0),
			WallDimensions {
				thickness: 0.1,
				height: 3.0,
			},
		)
		.expect("thinner, taller T stem");
	graph
		.add_wall_with_dimensions(
			corner,
			Vec2::new(14.0, 2.0),
			WallDimensions {
				thickness: 0.3,
				height: 3.2,
			},
		)
		.expect("thicker, taller corner wall");

	let stacked = graph
		.walls()
		.find(|&wall| graph.wall_position(wall, 0.0) == Some(through_start))
		.expect("through wall piece before the T");
	for bottom in [0.3, 1.3] {
		graph
			.add_opening(
				stacked,
				OpeningSpec {
					center_distance: 1.5,
					width: 0.8,
					bottom,
					height: 0.8,
				},
			)
			.expect("stacked window");
	}
}

fn add_visual_checks(graph: &mut WallGraph) {
	let major_arc = graph
		.add_arc(
			Vec2::new(-8.0, 8.0),
			Vec2::new(-10.0, 6.0),
			1.5 * std::f32::consts::PI,
		)
		.expect("major arc")[0];
	graph
		.add_opening(
			major_arc,
			OpeningSpec {
				center_distance: graph.wall_length(major_arc).unwrap() * 0.5,
				width: 0.8,
				bottom: 0.8,
				height: 1.0,
			},
		)
		.expect("major arc opening");

	graph
		.add_arc(
			Vec2::new(-5.0, 8.0),
			Vec2::new(-3.0, 10.0),
			std::f32::consts::FRAC_PI_2,
		)
		.expect("first joined arc");
	graph
		.add_arc_with_dimensions(
			Vec2::new(-3.0, 10.0),
			Vec2::new(-1.0, 8.0),
			-std::f32::consts::FRAC_PI_2,
			WallDimensions {
				thickness: 0.3,
				height: 3.0,
			},
		)
		.expect("mixed-width joined arc");

	for (origin, degrees, length) in [
		(Vec2::new(2.0, 8.0), 5.0f32, 6.0),
		(Vec2::new(11.0, 8.0), 20.0, 3.0),
		(Vec2::new(-5.0, 16.0), 30.0, 3.0),
		(Vec2::new(4.0, 16.0), 150.0, 3.0),
	] {
		graph
			.add_wall(origin, origin + Vec2::X * length)
			.expect("corner base");
		graph
			.add_wall(
				origin,
				origin + Vec2::from_angle(degrees.to_radians()) * length,
			)
			.expect("corner branch");
	}
}

fn debug_material(
	materials: &mut Assets<StandardMaterial>,
	color: Color,
) -> Handle<StandardMaterial> {
	materials.add(StandardMaterial {
		base_color: color,
		perceptual_roughness: 0.9,
		..default()
	})
}

fn surface_color(kind: SurfaceKind) -> Color {
	match kind {
		SurfaceKind::Side(wall_mesh::Side::Left) => Color::srgb(0.14, 0.66, 0.95),
		SurfaceKind::Side(wall_mesh::Side::Right) => Color::srgb(0.95, 0.30, 0.57),
		SurfaceKind::Top => Color::srgb(0.95, 0.87, 0.56),
		SurfaceKind::Bottom => Color::srgb(0.34, 0.38, 0.46),
		SurfaceKind::EndCap => Color::srgb(0.95, 0.61, 0.18),
		SurfaceKind::Jamb => Color::srgb(0.55, 0.90, 0.26),
		SurfaceKind::Head => Color::srgb(0.79, 0.41, 0.96),
		SurfaceKind::Sill => Color::srgb(0.18, 0.85, 0.61),
		SurfaceKind::JunctionStep => Color::srgb(0.96, 0.34, 0.21),
		SurfaceKind::Bevel => Color::srgb(1.0, 0.17, 0.13),
	}
}

fn source_wall(graph: &WallGraph, source: SurfaceSource) -> Option<Wall> {
	match source {
		SurfaceSource::Wall(wall) => Some(wall),
		SurfaceSource::Opening(id) => graph.opening(id).map(|opening| opening.wall),
		SurfaceSource::Junction(_) => None,
	}
}

fn setup(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let graph = build_demo_graph();
	let wall_mesh =
		wall_mesh::generate(&graph, &MeshSettings::default()).expect("demo walls can be meshed");

	let neutral = debug_material(&mut materials, Color::srgb(0.86, 0.84, 0.8));
	let junction = debug_material(&mut materials, Color::srgb(0.39, 0.42, 0.47));
	let mut wall_materials: HashMap<Wall, Handle<StandardMaterial>> = HashMap::new();
	let mut surface_materials: HashMap<SurfaceKind, Handle<StandardMaterial>> = HashMap::new();
	for (tag, mesh) in tagged_bevy_meshes(&wall_mesh) {
		let by_wall = if let Some(wall) = source_wall(&graph, tag.source) {
			let next_color = wall_materials.len();
			wall_materials
				.entry(wall)
				.or_insert_with(|| {
					let hue = (next_color as f32 * 137.508 + 25.0) % 360.0;
					debug_material(&mut materials, Color::hsl(hue, 0.72, 0.59))
				})
				.clone()
		} else {
			junction.clone()
		};
		let by_surface = surface_materials
			.entry(tag.kind)
			.or_insert_with(|| debug_material(&mut materials, surface_color(tag.kind)))
			.clone();
		commands.spawn((
			Mesh3d(meshes.add(mesh)),
			MeshMaterial3d(neutral.clone()),
			WallPiece {
				materials: [neutral.clone(), by_wall, by_surface],
			},
		));
	}
	commands.insert_resource(WireEdges(wire_edges(&wall_mesh)));
	commands.spawn((
		Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 32.0))),
		MeshMaterial3d(materials.add(StandardMaterial {
			base_color: Color::srgb(0.35, 0.4, 0.35),
			perceptual_roughness: 1.0,
			..default()
		})),
		// Slightly below the wall bottoms so the two do not fight for depth.
		Transform::from_xyz(4.0, -0.002, -7.0),
	));
	commands.spawn((
		DirectionalLight {
			illuminance: 8_000.0,
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_xyz(8.0, 12.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
	));
	commands.insert_resource(GlobalAmbientLight {
		brightness: 400.0,
		..default()
	});

	let (focus, distance) = InspectionArea::Overview.camera();
	let orbit = OrbitCamera {
		focus,
		distance,
		yaw: -0.5,
		pitch: 0.6,
	};
	commands.spawn((Camera3d::default(), orbit_transform(&orbit), orbit));
	spawn_toolbar(&mut commands);
	commands.insert_resource(DemoGraph(graph));
}

fn spawn_toolbar(commands: &mut Commands) {
	let buttons = [
		("Shaded", ToolbarAction::View(ViewMode::Shaded)),
		("Wall colors", ToolbarAction::View(ViewMode::ByWall)),
		("Surface colors", ToolbarAction::View(ViewMode::BySurface)),
		("Wireframe", ToolbarAction::View(ViewMode::Wireframe)),
		("House", ToolbarAction::Focus(InspectionArea::House)),
		("Arcs", ToolbarAction::Focus(InspectionArea::Arcs)),
		("Bevels", ToolbarAction::Focus(InspectionArea::Bevels)),
		("Corners", ToolbarAction::Focus(InspectionArea::Corners)),
		("Overview", ToolbarAction::Focus(InspectionArea::Overview)),
		("Edges", ToolbarAction::ToggleEdges),
		("Centerlines", ToolbarAction::ToggleCenterlines),
	];
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: px(8),
				left: px(8),
				right: px(8),
				flex_wrap: FlexWrap::Wrap,
				column_gap: px(5),
				row_gap: px(5),
				padding: UiRect::all(px(6)),
				..default()
			},
			BackgroundColor(Color::srgba(0.04, 0.06, 0.09, 0.85)),
		))
		.with_children(|toolbar| {
			for (label, action) in buttons {
				toolbar
					.spawn((
						Button,
						Node {
							justify_content: JustifyContent::Center,
							align_items: AlignItems::Center,
							padding: UiRect::axes(px(10), px(6)),
							border_radius: BorderRadius::all(px(4)),
							..default()
						},
						BackgroundColor(Color::srgb(0.18, 0.22, 0.28)),
						action,
					))
					.with_children(|button| {
						button.spawn((
							Text::new(label),
							TextFont {
								font_size: FontSize::Px(14.0),
								..default()
							},
							TextColor(Color::WHITE),
						));
					});
			}
		});
}

fn orbit_transform(orbit: &OrbitCamera) -> Transform {
	let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, -orbit.pitch, 0.0);
	Transform::from_translation(orbit.focus + rotation * Vec3::Z * orbit.distance)
		.looking_at(orbit.focus, Vec3::Y)
}

/// Left drag orbits, right drag pans, and the wheel zooms.
fn orbit_camera(
	buttons: Res<ButtonInput<MouseButton>>,
	motion: Res<AccumulatedMouseMotion>,
	scroll: Res<AccumulatedMouseScroll>,
	toolbar_buttons: Query<&Interaction, With<ToolbarAction>>,
	mut cameras: Query<(&mut OrbitCamera, &mut Transform)>,
) {
	if toolbar_buttons
		.iter()
		.any(|interaction| *interaction != Interaction::None)
	{
		return;
	}
	for (mut orbit, mut transform) in &mut cameras {
		if buttons.pressed(MouseButton::Left) {
			orbit.yaw -= motion.delta.x * 0.005;
			orbit.pitch = (orbit.pitch + motion.delta.y * 0.005).clamp(-0.2, 1.5);
		}
		if buttons.pressed(MouseButton::Right) {
			let right = transform.right().with_y(0.0).normalize_or_zero();
			let forward = transform.forward().with_y(0.0).normalize_or_zero();
			let scale = orbit.distance * 0.0015;
			orbit.focus += (-right * motion.delta.x + forward * motion.delta.y) * scale;
		}
		let notches = match scroll.unit {
			MouseScrollUnit::Line => scroll.delta.y,
			MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
		};
		orbit.distance = (orbit.distance * (1.0 - notches * 0.1)).clamp(1.5, 80.0);
		*transform = orbit_transform(&orbit);
	}
}

fn handle_toolbar(
	buttons: Query<(&Interaction, &ToolbarAction), Changed<Interaction>>,
	mut view: ResMut<ViewSettings>,
	mut centerlines: ResMut<ShowCenterlines>,
	mut cameras: Query<(&mut OrbitCamera, &mut Transform)>,
) {
	for (interaction, action) in &buttons {
		if *interaction != Interaction::Pressed {
			continue;
		}
		match action {
			ToolbarAction::View(mode) => view.mode = *mode,
			ToolbarAction::ToggleEdges => view.wire_overlay = !view.wire_overlay,
			ToolbarAction::ToggleCenterlines => centerlines.0 = !centerlines.0,
			ToolbarAction::Focus(area) => {
				let (focus, distance) = area.camera();
				for (mut orbit, mut transform) in &mut cameras {
					orbit.focus = focus;
					orbit.distance = distance;
					*transform = orbit_transform(&orbit);
				}
			}
		}
	}
}

fn apply_view_mode(
	view: Res<ViewSettings>,
	mut pieces: Query<(
		&WallPiece,
		&mut MeshMaterial3d<StandardMaterial>,
		&mut Visibility,
	)>,
) {
	if !view.is_changed() {
		return;
	}
	for (piece, mut material, mut visibility) in &mut pieces {
		*visibility = if view.mode == ViewMode::Wireframe {
			Visibility::Hidden
		} else {
			Visibility::Visible
		};
		let index = match view.mode {
			ViewMode::Shaded | ViewMode::Wireframe => 0,
			ViewMode::ByWall => 1,
			ViewMode::BySurface => 2,
		};
		material.0 = piece.materials[index].clone();
	}
}

fn update_toolbar_colors(
	view: Res<ViewSettings>,
	centerlines: Res<ShowCenterlines>,
	mut buttons: Query<(&Interaction, &ToolbarAction, &mut BackgroundColor)>,
) {
	for (interaction, action, mut background) in &mut buttons {
		let selected = match action {
			ToolbarAction::View(mode) => view.mode == *mode,
			ToolbarAction::ToggleEdges => view.wire_overlay,
			ToolbarAction::ToggleCenterlines => centerlines.0,
			ToolbarAction::Focus(_) => false,
		};
		background.0 = if *interaction == Interaction::Pressed {
			Color::srgb(0.18, 0.56, 0.70)
		} else if selected {
			Color::srgb(0.12, 0.44, 0.58)
		} else if *interaction == Interaction::Hovered {
			Color::srgb(0.28, 0.34, 0.42)
		} else {
			Color::srgb(0.18, 0.22, 0.28)
		};
	}
}

fn draw_wireframe(view: Res<ViewSettings>, edges: Res<WireEdges>, mut gizmos: Gizmos) {
	if view.mode != ViewMode::Wireframe && !view.wire_overlay {
		return;
	}
	let color = if view.mode == ViewMode::Wireframe {
		Color::srgb(1.0, 0.88, 0.25)
	} else {
		Color::srgb(0.04, 0.06, 0.09)
	};
	for &(start, end) in &edges.0 {
		gizmos.line(start, end, color);
	}
}

fn draw_centerlines(graph: Res<DemoGraph>, show: Res<ShowCenterlines>, mut gizmos: Gizmos) {
	if !show.0 {
		return;
	}
	let graph = &graph.0;
	for wall in graph.walls() {
		let lift = graph.wall_dimensions(wall).expect("live wall").height + CENTERLINE_LIFT;
		let points = graph.sample_wall(wall, 0.01).expect("valid demo wall");
		gizmos.linestrip(
			points
				.iter()
				.map(|point| graph_to_world(point.extend(lift))),
			Color::srgb(0.1, 0.4, 0.9),
		);
	}
	for (_, position) in graph.nodes() {
		gizmos.sphere(
			Isometry3d::from_translation(graph_to_world(position.extend(0.0))),
			0.08,
			Color::srgb(1.0, 0.4, 0.2),
		);
	}
}

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				name: Some("bevy-dev".into()),
				title: "House builder — wall mesh inspector".into(),
				..default()
			}),
			..default()
		}))
		.insert_resource(ShowCenterlines(true))
		.init_resource::<ViewSettings>()
		.add_systems(Startup, setup)
		.add_systems(
			Update,
			(
				orbit_camera,
				(handle_toolbar, apply_view_mode, update_toolbar_colors).chain(),
				draw_centerlines,
				draw_wireframe,
			),
		)
		.run();
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use wall_mesh::MeshSettings;

	use super::{
		InspectionArea, OrbitCamera, ShowCenterlines, ToolbarAction, ViewMode, ViewSettings,
	};

	#[test]
	fn toolbar_actions_update_the_view_and_camera() {
		let mut app = App::new();
		app.insert_resource(ViewSettings::default())
			.insert_resource(ShowCenterlines(true))
			.add_systems(Update, super::handle_toolbar);
		let camera = app
			.world_mut()
			.spawn((
				OrbitCamera {
					focus: Vec3::ZERO,
					distance: 1.0,
					yaw: 0.0,
					pitch: 0.0,
				},
				Transform::default(),
			))
			.id();
		for action in [
			ToolbarAction::View(ViewMode::BySurface),
			ToolbarAction::ToggleEdges,
			ToolbarAction::ToggleCenterlines,
			ToolbarAction::Focus(InspectionArea::Arcs),
		] {
			app.world_mut().spawn((Interaction::Pressed, action));
		}
		app.update();

		let view = app.world().resource::<ViewSettings>();
		assert!(view.mode == ViewMode::BySurface);
		assert!(view.wire_overlay);
		assert!(!app.world().resource::<ShowCenterlines>().0);
		let orbit = app.world().get::<OrbitCamera>(camera).unwrap();
		assert_eq!((orbit.focus, orbit.distance), InspectionArea::Arcs.camera());
	}

	#[test]
	fn demo_contains_curved_geometry_a_crossing_and_openings() {
		let graph = super::build_demo_graph();
		assert_eq!(graph.nodes().count(), 30);
		assert_eq!(graph.walls().count(), 23);
		assert_eq!(graph.openings().count(), 6);
		assert!(
			graph
				.openings()
				.any(|(_, opening)| graph.sample_wall(opening.wall, 0.01).unwrap().len() > 2)
		);
		assert!(
			graph
				.walls()
				.any(|wall| graph.sample_wall(wall, 0.01).unwrap().len() > 2)
		);
	}

	#[test]
	fn demo_walls_mesh_successfully() {
		let mesh = wall_mesh::generate(&super::build_demo_graph(), &MeshSettings::default())
			.expect("demo meshes");
		assert!(!mesh.is_empty());
		for kind in [
			wall_mesh::SurfaceKind::Bevel,
			wall_mesh::SurfaceKind::JunctionStep,
			wall_mesh::SurfaceKind::Jamb,
			wall_mesh::SurfaceKind::Head,
			wall_mesh::SurfaceKind::Sill,
		] {
			assert!(
				mesh.sections()
					.iter()
					.any(|section| section.tag.kind == kind)
			);
		}
	}
}
