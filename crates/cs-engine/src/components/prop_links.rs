//! Relationships SimulIDE applies when one property changes another.
//!
//! Each component declares these as `PropUpdate` rules. The setter stores only
//! its own field. The rule writes the fields that have to follow.

/// `Dialed::updtValue`: the current value stays inside the limits.
/// The value does not push a limit outward.
pub(crate) fn dialed_value(value: &mut f64, min: f64, max: f64) {
    if *value > max {
        *value = max;
    } else if *value < min {
        *value = min;
    }
}

/// `Dialed::setMinVal` / `VarResistor::setMinVal`: minimum yields to maximum,
/// then the current value is pulled into the range.
pub(crate) fn dialed_min(min: &mut f64, max: f64, value: &mut f64) {
    if *min > max {
        *min = max;
    }
    dialed_value(value, *min, max);
}

/// `Dialed::setMaxVal`: maximum yields to minimum, then the value is pulled in.
pub(crate) fn dialed_max(max: &mut f64, min: f64, value: &mut f64) {
    if *max < min {
        *max = min;
    }
    dialed_value(value, min, *max);
}

/// `VarSource::setVal`: a typed output outside the range moves that end.
pub(crate) fn source_value(value: f64, min: &mut f64, max: &mut f64) {
    if value > *max {
        *max = value;
    } else if value < *min {
        *min = value;
    }
}

/// `VarSource::setMaxValue`: maximum stays just above minimum, then output clamps.
pub(crate) fn source_max(max: &mut f64, min: f64, value: &mut f64, abs_max: f64) {
    if *max < min {
        let bumped = min + 1e-3;
        *max = if bumped <= abs_max {
            bumped
        } else {
            min.min(abs_max)
        };
    }
    if *value > *max {
        *value = *max;
    } else if *value < min {
        *value = min;
    }
}

/// `VarSource::setMinValue`: minimum stays just below maximum, then output clamps.
pub(crate) fn source_min(min: &mut f64, max: f64, value: &mut f64, abs_min: f64) {
    if *min > max {
        let bumped = max - 1e-3;
        *min = if bumped >= abs_min {
            bumped
        } else {
            max.max(abs_min)
        };
    }
    if *value < *min {
        *value = *min;
    } else if *value > max {
        *value = max;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialed_limit_yields_and_pulls_the_value() {
        let mut min = 80.0;
        let mut value = 10.0;
        dialed_min(&mut min, 50.0, &mut value);
        assert_eq!(min, 50.0);
        assert_eq!(value, 50.0);

        let mut max = 5.0;
        let mut value = 40.0;
        dialed_max(&mut max, 20.0, &mut value);
        assert_eq!(max, 20.0);
        assert_eq!(value, 20.0);
    }

    #[test]
    fn source_value_expands_the_range_it_leaves() {
        let mut min = 0.0;
        let mut max = 5.0;
        source_value(12.0, &mut min, &mut max);
        assert_eq!(max, 12.0);
        assert_eq!(min, 0.0);
        source_value(-2.0, &mut min, &mut max);
        assert_eq!(min, -2.0);
    }

    #[test]
    fn source_limit_does_not_cross_and_clamps_output() {
        let mut max = 1.0;
        let mut value = 4.0;
        source_max(&mut max, 3.0, &mut value, 1e6);
        assert_eq!(max, 3.001);
        assert_eq!(value, 3.001);

        let mut min = 9.0;
        let mut value = 1.0;
        source_min(&mut min, 4.0, &mut value, -1e6);
        assert_eq!(min, 3.999);
        assert_eq!(value, 3.999);
    }
}
