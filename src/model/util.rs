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

pub fn round_to_nearest_cent(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

pub fn gen_price_in_range(range: (f64, f64), cash: f64) -> Option<f64> {
    let (min, max) = range;
    let mut price = min;
    if min == max {
        price = min;
    } else if min > max {
        panic!("min {:} must be less than or equal to max {:}", min, max);
    } else {
        let mut rng = rand::thread_rng();
        price = rng.gen_range(min..max);
    }

    let price = if price > cash {
        if (min..max).contains(&cash) {
            Some(cash)
        } else {
            None
        }
    } else {
        Some(price)
    };
    if let Some(price) = price {
        if price < 0.01 {
            None
        } else {
            Some(round_to_nearest_cent(price))
        }
    } else {
        None
    }
}

pub fn gen_new_range_with_price(price: f64, old_range: (f64, f64), shrink_rate: f64) -> (f64, f64) {
    let (old_min, old_max) = old_range;
    let width = round_to_nearest_cent(old_max - old_min);
    let new_half_width = round_to_nearest_cent((width / 2.0) * shrink_rate);
    let new_min = round_to_nearest_cent(price - new_half_width).max(0.0);
    let mut new_max = round_to_nearest_cent(price + new_half_width);
    if new_max <= new_min {
        new_max = new_min + 0.01;
    }
    (new_min, new_max)
}

pub fn shift_range_by_ratio(old_range: (f64, f64), rate: f64) -> (f64, f64) {
    let mut new_max = round_to_nearest_cent(old_range.1 * (1.0 + rate));
    let new_min = round_to_nearest_cent((old_range.0 * (1.0 + rate)).max(0.0));
    if new_max <= new_min {
        new_max = new_min + 0.01;
    }
    (new_min,new_max)
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

    #[test]
    fn test_gen_price_in_range() {
        // 测试正常情况：生成的价格≤现金
        for _ in 0..100 {
            let range = (10.0, 20.0);
            let cash = 30.0;
            let result = gen_price_in_range(range, cash);
            assert!(
                result.is_some(),
                "Result should be Some when cash is sufficient"
            );
            let price = result.unwrap();
            assert!(
                price >= range.0,
                "Price should be >= range min: {} >= {}",
                price,
                range.0
            );
            assert!(
                price <= range.1,
                "Price should be <= range max: {} <= {}",
                price,
                range.1
            );
            assert!(
                price <= cash,
                "Price should be <= cash: {} <= {}",
                price,
                cash
            );
        }

        // 测试价格>现金，但现金在范围内
        // 固定范围和现金，确保现金在范围内但小于随机生成的价格
        let range = (10.0, 20.0);
        let cash = 15.0;
        // 由于是随机生成，我们多次尝试，确保覆盖到价格>现金的情况
        let mut success_count = 0;
        let mut cases_within_range = 0;
        for _ in 0..1000 {
            let result = gen_price_in_range(range, cash);
            assert!(
                result.is_some(),
                "Result should be Some when cash is in range"
            );
            let price = result.unwrap();
            assert!(
                price >= range.0,
                "Price should be >= range min: {} >= {}",
                price,
                range.0
            );
            assert!(
                price <= range.1,
                "Price should be <= range max: {} <= {}",
                price,
                range.1
            );
            if price == cash {
                success_count += 1;
            }
            cases_within_range += 1;
        }
        assert!(
            cases_within_range > 0,
            "Should have cases where price > cash but cash is in range"
        );
        assert!(
            success_count > 0,
            "Should have cases where price is set to cash when price > cash"
        );

        // 测试价格>现金，且现金不在范围内
        let range = (10.0, 20.0);
        let cash = 5.0; // 现金小于范围最小值
        let result = gen_price_in_range(range, cash);
        assert!(
            result.is_none(),
            "Result should be None when cash is below range"
        );

        let range = (10.0, 20.0);
        let cash = 25.0; // 现金大于范围最大值
        let result = gen_price_in_range(range, cash);
        // 这种情况应该返回Some，因为生成的价格会小于等于现金（范围上限20.0 < 25.0）
        assert!(
            result.is_some(),
            "Result should be Some when cash is above range"
        );

        // 测试最小价格等于最大价格的情况
        let range = (15.0, 15.0);
        let cash = 20.0;
        let result = gen_price_in_range(range, cash);
        assert!(
            result.is_some(),
            "Result should be Some when range is a single point"
        );
        let price = result.unwrap();
        assert_eq!(
            price, 15.0,
            "Price should be equal to range value when range is a single point"
        );

        // 测试现金等于最小价格的情况
        let range = (10.0, 20.0);
        let cash = 10.0;
        for _ in 0..100 {
            let result = gen_price_in_range(range, cash);
            if let Some(price) = result {
                assert!(
                    price >= cash,
                    "Price should be >= cash when cash equals range min"
                );
                assert!(price <= range.1, "Price should be <= range max");
            } else {
                // 如果生成的价格>现金，由于现金等于范围最小值，应该返回cash
                assert!(false, "Result should be Some when cash equals range min");
            }
        }

        // 测试现金等于最大价格的情况
        let range = (10.0, 20.0);
        let cash = 20.0;
        for _ in 0..100 {
            let result = gen_price_in_range(range, cash);
            assert!(
                result.is_some(),
                "Result should be Some when cash equals range max"
            );
            let price = result.unwrap();
            assert!(price >= range.0, "Price should be >= range min");
            assert!(
                price <= cash,
                "Price should be <= cash when cash equals range max"
            );
        }

        // 测试现金为0的情况
        let range = (10.0, 20.0);
        let cash = 0.0;
        let result = gen_price_in_range(range, cash);
        assert!(
            result.is_none(),
            "Result should be None when cash is 0 and below range"
        );

        // 测试边界情况：价格范围非常小
        let range = (10.0, 10.01);
        let cash = 15.0;
        for _ in 0..100 {
            let result = gen_price_in_range(range, cash);
            assert!(
                result.is_some(),
                "Result should be Some when range is very small"
            );
            let price = result.unwrap();
            assert!(price >= range.0, "Price should be >= range min");
            assert!(price <= range.1, "Price should be <= range max");
            assert!(price >= 0.01, "Price should be >= 0.01: {}", price);
        }

        // 测试价格<0.01的情况：应返回None
        let range = (0.0, 0.005);
        let cash = 0.01;
        let result = gen_price_in_range(range, cash);
        assert!(result.is_none(), "Result should be None when price < 0.01");

        // 测试价格刚好等于0.01的情况：应返回Some(0.01)
        let range = (0.01, 0.01);
        let cash = 0.02;
        let result = gen_price_in_range(range, cash);
        assert!(result.is_some(), "Result should be Some when price = 0.01");
        let price = result.unwrap();
        assert_eq!(
            price, 0.01,
            "Price should be 0.01 when range is (0.01, 0.01)"
        );

        // 测试价格>0.01的情况：应返回Some(price)
        let range = (0.01, 0.02);
        let cash = 0.03;
        let result = gen_price_in_range(range, cash);
        assert!(result.is_some(), "Result should be Some when price > 0.01");
        let price = result.unwrap();
        assert!(price >= 0.01, "Price should be >= 0.01: {}", price);
        assert!(price <= 0.02, "Price should be <= 0.02: {}", price);

        // 测试现金<0.01且在范围内的情况
        let range = (0.0, 0.005);
        let cash = 0.002;
        let result = gen_price_in_range(range, cash);
        assert!(
            result.is_none(),
            "Result should be None when cash < 0.01 and in range"
        );

        // 测试现金=0.01且在范围内的情况（range包含0.01）
        let range = (0.0, 0.02);
        let cash = 0.01;
        // 多次尝试，确保覆盖到价格>cash的情况
        let mut success_count = 0;
        let mut total_attempts = 0;
        for _ in 0..1000 {
            let result = gen_price_in_range(range, cash);
            total_attempts += 1;
            if let Some(price) = result {
                assert!(price >= 0.01, "Price should be >= 0.01: {}", price);
                assert!(price <= 0.02, "Price should be <= 0.02: {}", price);
                // todo 验证是两位小数
                if price == cash {
                    success_count += 1;
                }
            }
        }
        assert!(total_attempts > 0, "Should have made attempts");
        assert!(
            success_count > 0,
            "Should have cases where price is set to cash"
        );

        // 测试价格四舍五入到两位小数
        let range = (10.0, 20.0);
        let cash = 30.0;
        for _ in 0..100 {
            let result = gen_price_in_range(range, cash);
            assert!(
                result.is_some(),
                "Result should be Some when cash is sufficient"
            );
            let price = result.unwrap();
            // 验证价格是两位小数
            let decimal_places = format!("{:.20}", price)
                .split('.')
                .nth(1)
                .unwrap_or("")
                .len();
        }

        // 测试边界情况：价格正好在分的边界上
        let range = (10.0, 10.0);
        let cash = 20.0;
        let result = gen_price_in_range(range, cash);
        assert!(
            result.is_some(),
            "Result should be Some when range is (10.0, 10.0)"
        );
        let price = result.unwrap();
        assert_eq!(price, 10.0, "Price should be 10.0 for range (10.0, 10.0)");
        let decimal_places = format!("{:.20}", price)
            .split('.')
            .nth(1)
            .unwrap_or("")
            .len();

        // 测试价格需要向上取整的情况
        let range = (10.014, 10.015);
        let cash = 20.0;
        let result = gen_price_in_range(range, cash);
        assert!(
            result.is_some(),
            "Result should be Some for range (10.014, 10.015)"
        );
        let price = result.unwrap();
        // 10.014 应四舍五入为 10.01，10.015 应四舍五入为 10.02
        assert!(
            price == 10.01 || price == 10.02,
            "Price should be either 10.01 or 10.02: {}",
            price
        );
        let decimal_places = format!("{:.20}", price)
            .split('.')
            .nth(1)
            .unwrap_or("")
            .len();

        // 测试价格需要向下取整的情况
        let range = (10.011, 10.012);
        let cash = 20.0;
        let result = gen_price_in_range(range, cash);
        assert!(
            result.is_some(),
            "Result should be Some for range (10.011, 10.012)"
        );
        let price = result.unwrap();
        // 10.011 和 10.012 都应四舍五入为 10.01
        assert_eq!(
            price, 10.01,
            "Price should be 10.01 for range (10.011, 10.012): {}",
            price
        );
        let decimal_places = format!("{:.20}", price)
            .split('.')
            .nth(1)
            .unwrap_or("")
            .len();
    }

    #[test]
    fn test_gen_new_range_with_price() {
        // 测试1：正常情况：生成有效的新范围
        let old_range = (10.0, 20.0); // 宽度10.0
        let price = 15.0;
        let shrink_rate = 0.8;
        let (new_min, new_max) = gen_new_range_with_price(price, old_range, shrink_rate);
        assert!(new_max > new_min, "New range should be valid");
        assert!(new_min >= 0.0, "New min should be non-negative");

        // 测试2：不同的shrink_rate值
        let shrink_rates = [0.5, 0.7, 0.9, 0.99];
        for &rate in shrink_rates.iter() {
            let (new_min, new_max) = gen_new_range_with_price(price, old_range, rate);
            assert!(
                new_max > new_min,
                "New range should be valid for shrink_rate {}",
                rate
            );
            assert!(
                new_min >= 0.0,
                "New min should be non-negative for shrink_rate {}",
                rate
            );
        }

        // 测试3：旧范围宽度很小的情况
        let old_range = (10.0, 10.01); // 宽度0.01
        let price = 10.005;
        let (new_min, new_max) = gen_new_range_with_price(price, old_range, 0.9);
        assert!(
            new_max > new_min,
            "New range should be valid for small old range"
        );
        assert!(
            new_min >= 0.0,
            "New min should be non-negative for small old range"
        );

        // 测试4：新范围以给定价格为中心
        let old_range = (5.0, 15.0); // 宽度10.0
        let price = 10.0;
        let shrink_rate = 0.8;
        let (new_min, new_max) = gen_new_range_with_price(price, old_range, shrink_rate);
        // 计算中心偏移量，允许一定误差（由于四舍五入）
        let center = (new_min + new_max) / 2.0;
        let center_offset = (center - price).abs();
        assert!(
            center_offset < 0.01,
            "New range should be centered at price: center={}, price={}",
            center,
            price
        );

        // 测试5：新范围比旧范围小
        let old_width = old_range.1 - old_range.0;
        let new_width = new_max - new_min;
        assert!(
            new_width < old_width,
            "New range should be smaller than old range: new_width={}, old_width={}",
            new_width,
            old_width
        );

        // 测试6：价格为边界值的情况
        let old_range = (0.0, 10.0);
        let price = 0.0;
        let (new_min, new_max) = gen_new_range_with_price(price, old_range, 0.5);
        assert!(
            new_max > new_min,
            "New range should be valid for price at boundary"
        );
        assert!(
            new_min >= 0.0,
            "New min should be non-negative for price at boundary"
        );

        // 测试7：shrink_rate为1.0的情况（新范围宽度与旧范围相同）
        let shrink_rate = 1.0;
        let (new_min, new_max) = gen_new_range_with_price(23.1, (20.0, 30.0), shrink_rate);
        assert!(
            new_max > new_min,
            "New range should be valid for shrink_rate=1.0"
        );
        let new_width = new_max - new_min;
        let old_width = old_range.1 - old_range.0;
        println!("old_range={:?}", (20.0, 30.0));
        println!("new_range={:?}", (new_min, new_max));
        println!("new_width={}, old_width={}", new_width, old_width);
        // 由于四舍五入，可能存在0.01的误差
        assert!(
            (new_width - old_width).abs() < 0.02,
            "New width should be approximately equal to old width for shrink_rate=1.0: new_width={}, old_width={}",
            new_width,
            old_width
        );

        // 测试8：极端情况：shrink_rate非常小
        let shrink_rate = 0.01;
        let (new_min, new_max) = gen_new_range_with_price(23.4, (20.0, 30.0), shrink_rate);
        println!("old_range:{:?}", (20.0, 30.0));
        println!("new_min:{:?} new_max:{:?}", new_min, new_max);
        assert!(
            new_max > new_min,
            "New range should be valid for very small shrink_rate"
        );
        assert!(
            new_min >= 0.0,
            "New min should be non-negative for very small shrink_rate"
        );
        let new_width = new_max - new_min;
        let rate = new_width / (old_range.1 - old_range.0);
        assert!(
            rate < 0.02,
            "New range should be very small for very small shrink_rate: rate={}",
            rate
        );
    }
}
