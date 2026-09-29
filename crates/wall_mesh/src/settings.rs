use crate::MeshError;

/// Options for [`generate`](crate::generate). Distances are in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshSettings {
	/// Largest distance between a curved wall surface and its flat triangles.
	pub max_deviation: f64,
	/// Longest outside miter, as a multiple of the thicker wall's half-thickness.
	/// Sharper corners are bevelled instead of forming a spike.
	pub miter_limit: f64,
	/// Least wall length required between an opening and a resolved junction.
	pub min_pier_width: f64,
	/// Largest number of triangles a mesh may contain.
	pub max_triangles: usize,
}

impl Default for MeshSettings {
	fn default() -> Self {
		Self {
			max_deviation: 0.005,
			miter_limit: 4.0,
			min_pier_width: 0.05,
			max_triangles: 2_000_000,
		}
	}
}

impl MeshSettings {
	pub(crate) fn validate(&self) -> Result<(), MeshError> {
		let valid = self.max_deviation.is_finite()
			&& self.max_deviation > 0.0
			&& self.miter_limit.is_finite()
			&& self.miter_limit >= 1.0
			&& self.min_pier_width.is_finite()
			&& self.min_pier_width >= 0.0
			&& self.max_triangles > 0;
		if valid {
			Ok(())
		} else {
			Err(MeshError::InvalidSettings)
		}
	}
}
