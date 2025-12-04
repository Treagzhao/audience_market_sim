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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_interval_intersection_overlap() {
        // 测试完全重叠
        assert_eq!(interval_intersection((1.0, 5.0), (1.0, 5.0)), Some((1.0, 5.0)));
        
        // 测试部分重叠 - 第一个区间在第二个区间左边
        assert_eq!(interval_intersection((1.0, 3.0), (2.0, 4.0)), Some((2.0, 3.0)));
        
        // 测试部分重叠 - 第一个区间在第二个区间右边
        assert_eq!(interval_intersection((2.0, 4.0), (1.0, 3.0)), Some((2.0, 3.0)));
        
        // 测试一个区间包含另一个区间
        assert_eq!(interval_intersection((1.0, 5.0), (2.0, 3.0)), Some((2.0, 3.0)));
        
        // 测试一个区间包含另一个区间（反向）
        assert_eq!(interval_intersection((2.0, 3.0), (1.0, 5.0)), Some((2.0, 3.0)));
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
        assert_eq!(interval_intersection((1.0, 2.0), (2.0, 3.0)), Some((2.0, 2.0)));
        
        // 测试两个区间相邻（反向），在端点处有交集
        assert_eq!(interval_intersection((2.0, 3.0), (1.0, 2.0)), Some((2.0, 2.0)));
    }
    
    #[test]
    fn test_interval_intersection_same_point() {
        // 测试两个区间都是同一点
        assert_eq!(interval_intersection((2.0, 2.0), (2.0, 2.0)), Some((2.0, 2.0)));
        
        // 测试一个区间是点，另一个包含该点
        assert_eq!(interval_intersection((2.0, 2.0), (1.0, 3.0)), Some((2.0, 2.0)));
        
        // 测试一个区间是点，另一个不包含该点
        assert_eq!(interval_intersection((2.0, 2.0), (3.0, 4.0)), None);
    }
}