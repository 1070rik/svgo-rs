mod traits;
mod path;
mod transform;
mod common_transform;
mod remove_common_transform;

pub use traits::SVGPlugin;
pub use path::PathOptimizerPlugin;
pub use transform::TransformOptimizerPlugin;
pub use common_transform::CommonTransformOptimizer;
pub use remove_common_transform::RemoveCommonTransformOptimizer;
