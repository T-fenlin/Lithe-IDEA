pub const ROW_RADIUS: f32 = 4.;

pub const ROW_LINE_HEIGHT: f32 = 1.35;

pub const TAB_MAX_WIDTH: f32 = 200.;

pub const CONTROL_RADIUS: f32 = 6.4;

pub const ACTIVITY_RAIL_WIDTH_SPEC: f32 = 40.;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_rail_width_matches_the_macos_source() {
        assert_eq!(ACTIVITY_RAIL_WIDTH_SPEC, 40.);
    }
}
