pub fn avg(data: &[u128]) -> f32 {
    let len = data.len();
    if len == 0 {
        return 0.0;
    }

    data.iter().sum::<u128>() as f32 / len as f32
}

pub fn percentile(data: &mut[u128], percent: f64) -> f64 {
    assert!(100.0 <= percent || percent > 0.0, "Percentage is out of bounds");

    let len = data.len();
    assert!(len > 0, "List can not be empty");

    if len == 1 {
        return data[0] as f64;
    }

    data.sort();
    if percent == 100.0 {
        return  *data.last().unwrap() as f64;
    }

    let idx = (len - 1) as f64 * percent/100.0;
    let floor = idx.floor();
    let ceil = idx.ceil();
    if floor == ceil {
        return data[idx as usize] as f64;
    }

    (data[floor as usize] as f64 * (ceil - idx)) + (data[ceil as usize] as f64 * (idx - floor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avg() {
        assert_eq!(avg(&[2, 1]), 1.5);
        assert_eq!(avg(&[1]), 1.0);
        assert_eq!(avg(&[]), 0.0);
        assert_eq!(avg(&[1, 4, 5, 10, 100]), 24.0);
    }

    #[test]
    fn test_percentile() {
        assert_eq!(percentile(&mut [2, 1], 50.0), 1.5);
        assert_eq!(percentile(&mut [1], 40.0), 1.);
        assert_eq!(percentile(&mut [1, 4, 5, 10, 99], 100.00), 99.);
        assert_eq!(percentile(&mut [1, 4, 5, 10, 99], 95.00), 81.19999999999999);
        assert_eq!(percentile(&mut [1, 4, 99, 5, 10], 95.00), 81.19999999999999);

        let mut data = vec![33, 33, 33, 33, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 35, 35, 35, 35, 35, 36, 37, 37, 38, 39, 40, 43, 47, 48, 48, 49, 54, 54, 54, 54, 54, 54];
        assert_eq!(percentile(&mut data, 50.00), 35.);
    }

    #[test]
    #[should_panic(expected = "List can not be empty")]
    fn test_percentile_panics() {
        percentile(&mut [], 80.0);
    }
}
