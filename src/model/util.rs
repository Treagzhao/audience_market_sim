use rand::Rng;

/// 计算两个区间的交集
/// 输入两个区间 (a1, a2) 和 (b1, b2)
/// 如果有交集，返回 Some((交集左端点, 交集右端点))
/// 如果没有交集，返回 None
pub fn interval_intersection(interval1: (f64, f64), interval2: (f64, f64)) -> Option<(f64, f64)> {
    // 确保区间的左端点 <= 右端点
    let (a1, a2) = interval1;
    let (b1, b2) = interval2;

    // 交集的左端点是两个区间左端点的最大值
    let left = a1.max(b1);
    // 交集的右端点是两个区间右端点的最小值
    let right = a2.min(b2);

    // 如果左端点 <= 右端点，则存在交集
    if left <= right {
        Some((left, right))
    } else {
        None
    }
}

/// 生成随机范围
/// 输入：
/// - min: 期望的最小下限
/// - max: 期望的最大上限
/// 输出：
/// - (f64, f64): 随机生成的范围，确保：
///   1. 下限 >= 0.0
///   2. 上限 > 下限
///   3. 上限 <= max
pub fn generate_random_range(min: f64, max: f64) -> (f64, f64) {
    let mut rng = rand::thread_rng();

    // 确保max不小于0.0
    let max = max.max(0.0);
    // 确保min不小于0.0且不大于max
    let min = min.max(0.0).min(max);

    // 确保生成的范围有效且不超过max
    let (range_min, range_max) = if max <= min {
        // 如果max <= min，生成一个默认的小范围，确保上限不超过max
        (0.0, max.max(0.01))
    } else if max - min < 0.01 {
        // 如果范围太小，在有效范围内生成随机范围
        // 确保不超过max
        let range_min = min;
        let range_max = if min < max { max } else { min + 0.01 };
        (range_min, range_max)
    } else {
        // 正常情况：生成随机下限和上限
        // 确保生成的范围是有效的
        let max_possible_min = max * 0.5;

        let range_min = if max_possible_min <= min {
            // 如果max_possible_min <= min，使用min作为下限
            min
        } else {
            // 否则在min到max_possible_min之间随机生成
            rng.gen_range(min..max_possible_min)
        };

        let range_max = rng.gen_range(range_min..max);

        (range_min, range_max)
    };

    // 确保最终结果满足所有条件
    let final_min = range_min.max(0.0);
    let final_max = if range_max <= final_min {
        // 确保上限大于下限
        // 当max为0时，我们需要特殊处理，确保上限大于下限
        let candidate_max = final_min + 0.01;
        // 只有当max > 0时才应用max限制
        if max > 0.0 {
            candidate_max.min(max)
        } else {
            candidate_max
        }
    } else {
        // 确保上限大于下限
        // 当max为0时，我们需要特殊处理，确保上限大于下限
        if max > 0.0 {
            // 正常情况：确保上限不超过max
            range_max.min(max)
        } else {
            // 特殊情况：max为0，确保上限大于下限
            if range_max > final_min {
                range_max
            } else {
                final_min + 0.01
            }
        }
    };

    (final_min, final_max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_intersection_overlap() {
        // 测试完全重叠
        assert_eq!(
            interval_intersection((1.0, 5.0), (1.0, 5.0)),
            Some((1.0, 5.0))
        );

        // 测试部分重叠 - 第一个区间在第二个区间左边
        assert_eq!(
            interval_intersection((1.0, 3.0), (2.0, 4.0)),
            Some((2.0, 3.0))
        );

        // 测试部分重叠 - 第一个区间在第二个区间右边
        assert_eq!(
            interval_intersection((2.0, 4.0), (1.0, 3.0)),
            Some((2.0, 3.0))
        );

        // 测试一个区间包含另一个区间
        assert_eq!(
            interval_intersection((1.0, 5.0), (2.0, 3.0)),
            Some((2.0, 3.0))
        );

        // 测试一个区间包含另一个区间（反向）
        assert_eq!(
            interval_intersection((2.0, 3.0), (1.0, 5.0)),
            Some((2.0, 3.0))
        );
    }

    #[test]
    fn test_interval_intersection_no_overlap() {
        // 测试第一个区间在第二个区间左边，没有重叠
        assert_eq!(interval_intersection((1.0, 2.0), (3.0, 4.0)), None);

        // 测试第一个区间在第二个区间右边，没有重叠
        assert_eq!(interval_intersection((3.0, 4.0), (1.0, 2.0)), None);
    }

    #[test]
    fn test_interval_intersection_touching() {
        // 测试两个区间相邻，在端点处有交集
        assert_eq!(
            interval_intersection((1.0, 2.0), (2.0, 3.0)),
            Some((2.0, 2.0))
        );

        // 测试两个区间相邻（反向），在端点处有交集
        assert_eq!(
            interval_intersection((2.0, 3.0), (1.0, 2.0)),
            Some((2.0, 2.0))
        );
    }

    #[test]
    fn test_interval_intersection_same_point() {
        // 测试两个区间都是同一点
        assert_eq!(
            interval_intersection((2.0, 2.0), (2.0, 2.0)),
            Some((2.0, 2.0))
        );

        // 测试一个区间是点，另一个包含该点
        assert_eq!(
            interval_intersection((2.0, 2.0), (1.0, 3.0)),
            Some((2.0, 2.0))
        );

        // 测试一个区间是点，另一个不包含该点
        assert_eq!(interval_intersection((2.0, 2.0), (3.0, 4.0)), None);
    }

    #[test]
    fn test_generate_random_range_normal() {
        // 测试正常情况
        for _ in 0..100 {
            let (min, max) = generate_random_range(0.0, 100.0);
            assert!(min >= 0.0, "min should be non-negative: {}", min);
            assert!(
                max > min,
                "max should be greater than min: {} > {}",
                max,
                min
            );
            assert!(max <= 100.0, "max should be <= 100.0: {}", max);
        }
    }

    #[test]
    fn test_generate_random_range_negative_min() {
        // 测试负下限情况
        for _ in 0..100 {
            let (min, max) = generate_random_range(-100.0, 100.0);
            assert!(
                min >= 0.0,
                "min should be non-negative even when input min is negative: {}",
                min
            );
            assert!(
                max > min,
                "max should be greater than min: {} > {}",
                max,
                min
            );
            assert!(max <= 100.0, "max should be <= 100.0: {}", max);
        }
    }

    #[test]
    fn test_generate_random_range_small_range() {
        // 测试小范围情况
        for _ in 0..100 {
            let (min, max) = generate_random_range(50.0, 50.01);
            assert!(min >= 0.0, "min should be non-negative: {}", min);
            assert!(
                max > min,
                "max should be greater than min: {} > {}",
                max,
                min
            );
            assert!(max <= 50.01, "max should be <= 50.01: {}", max);
        }
    }

    #[test]
    fn test_generate_random_range_zero_max() {
        // 测试max为0的情况
        for _ in 0..100 {
            let (min, max) = generate_random_range(0.0, 0.0);
            assert!(min >= 0.0, "min should be non-negative: {}", min);
            assert!(
                max > min,
                "max should be greater than min: {} > {}",
                max,
                min
            );
            assert!(
                max <= 0.01,
                "max should be small when input max is 0: {}",
                max
            );
        }
    }

    #[test]
    fn test_generate_random_range_large_values() {
        // 测试大数值情况
        for _ in 0..100 {
            let (min, max) = generate_random_range(1000.0, 10000.0);
            assert!(min >= 0.0, "min should be non-negative: {}", min);
            assert!(
                max > min,
                "max should be greater than min: {} > {}",
                max,
                min
            );
            assert!(max <= 10000.0, "max should be <= 10000.0: {}", max);
        }
    }
}
