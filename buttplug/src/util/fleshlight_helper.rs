pub struct FleshlightHelper {}

impl FleshlightHelper {
    /// <summary>
    /// Speed returns the distance (in percent) moved given speed (in percent)
    /// in the given duration (milliseconds).
    /// Thanks to @funjack (https://github.com/funjack/launchcontrol/blob/master/protocol/funscript/functions.go).
    /// </summary>
    /// <param name="aDuration">The time to move in milliseconds.</param>
    /// <param name="aSpeed">The speed as a percentage (0.0-1.0).</param>
    /// <returns>The distance as a percentage (0.0-1.0).</returns>
    pub fn get_distance(duration: u32, speed: f64) -> f64
    {
        let mut s = speed;
        if s <= 0.0 {
            return 0.0;
        }

        if s > 1.0 {
            s = 1.0;
        }

        let mil: f64 = (s/250.0).powf(-0.95);
        let diff = mil - duration as f64;
        if diff.abs() < 0.001 { 0.0 } else { ((90.0 - (diff / mil * 90.0)) / 100.0).min(1.0).max(0.0) }
    }

    /// <summary>
    /// Speed returns the speed (in percent) to move the given distance (in percent)
    /// in the given duration (milliseconds).
    /// Thanks to @funjack (https://github.com/funjack/launchcontrol/blob/master/protocol/funscript/functions.go).
    /// </summary>
    /// <param name="aDistance">The distance as a percentage (0.0-1.0).</param>
    /// <param name="aDuration">The time to move in milliseconds.</param>
    /// <returns>The speed as a percentage (0.0-1.0).</returns>
    pub fn get_speed(distance: f64, duration: u32) -> f64 {

        let mut d = distance;
        if d <= 0.0 {
            return 0.0;
        }

        if d > 1.0 {
            d = 1.0;
        }

        250.0 * ((duration * 90) as f64 / (d * 100.0)).powf(-1.05)
    }

    /// <summary>
    /// Duration returns the time it will take to move the given distance (in
    /// percent) at the given speed (in percent).
    /// </summary>
    /// <param name="aDistance">The distance as a percentage (0.0-1.0).</param>
    /// <param name="aSpeed">The speed as a percentage (0.0-1.0).</param>
    /// <returns>The time it will take to move in milliseconds.</returns>
    pub fn get_duration(distance: f64, speed: f64) -> u32 {
        let mut d = distance;
        let mut s = speed;

        if d <= 0.0 {
            return 0;
        }

        if d > 1.0 {
            d = 1.0;
        }

        if s <= 0.0 {
            return 0;
        }

        if s > 1.0 {
            s = 1.0;
        }

        let mil = (s / 250.0).powf(-0.95);
        (mil / (90.0 / (d * 100.0))) as u32
    }
}
