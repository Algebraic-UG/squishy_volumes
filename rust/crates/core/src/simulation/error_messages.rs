pub const SAMPLING_FAILED: &'static str = "Object could not be sampled.\n\
This is most likely because the volume is negative or tiny relative to the resolution.\n\
Please check the normals and consider increasing the resolution by reducing 'Grid Node Size' or 'Particle Factor'.\n\
(To increase sample density)";
pub const INVERTED_PARTICLE: &'static str = "A particle collapsed or inverted.\n\
This is most likely because the 'Time Step' is too large or because colliders are crushing particles.\n\
Please try to reduce the 'Time Step' by half (repeatedly) until stability is achieved, then increase cautiously to regain performance.";
