use serde::Deserialize;

/// Attached input device calibration, as returned by the `deviceInfo`
/// query. All fields optional - the daemon may not know them.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    /// Input device name.
    pub name: Option<String>,
    /// Device units per millimeter on X.
    pub x_res: Option<f64>,
    /// Device units per millimeter on Y.
    pub y_res: Option<f64>,
    /// Minimum raw X value.
    pub x_min: Option<f64>,
    /// Maximum raw X value.
    pub x_max: Option<f64>,
    /// Minimum raw Y value.
    pub y_min: Option<f64>,
    /// Maximum raw Y value.
    pub y_max: Option<f64>,
}

impl DeviceInfo {
    /// Physical screen size in millimeters, if the daemon reports axis
    /// resolution and range.
    pub fn screen_mm(&self) -> Option<(f64, f64)> {
        let x = (self.x_max? - self.x_min?) / self.x_res?;
        let y = (self.y_max? - self.y_min?) / self.y_res?;
        Some((x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::DeviceInfo;

    #[test]
    fn deserializes_camel_case_fields() {
        let info: DeviceInfo =
            serde_json::from_str(r#"{"name":"iiyama TF6539UHSC","xRes":2.5,"yRes":2.5,"xMin":0.0,"xMax":3500.0,"yMin":0.0,"yMax":2000.0}"#).expect("decode");
        let (w, h) = info.screen_mm().expect("mm");
        assert!((w - 1400.0).abs() < f64::EPSILON);
        assert!((h - 800.0).abs() < f64::EPSILON);
    }
}
