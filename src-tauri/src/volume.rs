//! Reusable CCP-static-data volume calculations. Unit volume is never
//! invented here: unknown CCP volume always propagates as unknown.

pub fn volume_for_quantity(unit_volume_m3: Option<f64>, quantity: i64) -> Option<f64> {
    if quantity < 0 {
        return None;
    }
    unit_volume_m3.map(|unit| unit * quantity as f64)
}

pub fn aggregate_volume(values: impl IntoIterator<Item = Option<f64>>) -> Option<f64> {
    let mut total = 0.0;
    let mut found = false;
    for value in values {
        total += value?;
        found = true;
    }
    found.then_some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantity_volume_uses_ccp_unit_volume() {
        assert_eq!(volume_for_quantity(Some(1.5), 4), Some(6.0));
        assert_eq!(volume_for_quantity(None, 4), None);
    }

    #[test]
    fn aggregate_volume_is_complete_or_unknown() {
        assert_eq!(aggregate_volume([Some(2.0), Some(3.5)]), Some(5.5));
        assert_eq!(aggregate_volume([Some(2.0), None]), None);
        assert_eq!(aggregate_volume(std::iter::empty()), None);
    }
}
