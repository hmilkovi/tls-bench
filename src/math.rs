pub fn median_nonempty(list: &mut [u128]) -> f64 {
    let len = list.len();
    assert!(len > 0, "List can not be empty");
    if len % 2 == 0 {
        let (_, &mut x, rest) = list.select_nth_unstable(len / 2 - 1);
        let (_, &mut y, _) = rest.select_nth_unstable(0);
        (x as f64 + y as f64) / 2.0
    } else {
        let (_, &mut mid, _) = list.select_nth_unstable(len / 2);
        mid as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_median_nonempty() {
        assert_eq!(median_nonempty(&mut [1]), 1.);
        assert_eq!(median_nonempty(&mut [2, 1]), 1.5);
        assert_eq!(median_nonempty(&mut [3, 1, 2]), 2.);
    }

    #[test]
    #[should_panic(expected = "List can not be empty")]
    fn test_median_nonempty_panics() {
        median_nonempty(&mut []);
    }
}
