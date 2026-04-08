use crate::model::product::Product;
use rand::Rng;
#[derive(Clone,Debug)]
pub struct Preference {
    pub original_price: f64,
    pub original_elastic: f64,
    pub(crate) current_price: f64,
    pub(crate) current_range: (f64, f64),
}

impl Preference {
    pub fn new(original_price: f64, original_elastic: f64) -> Self {
        Preference {
            original_price,
            original_elastic,
            current_price: 0.0,
            current_range: (0.0, 0.0),
        }
    }

    pub fn from_product(product: &Product) -> Self {
        // 使用产品的价格分布生成原始价格
        let original_price = product
            .original_price_distribution()
            .sample(Some((0.01, 1000000.0)));
        // 使用产品的弹性分布生成原始弹性，并限制在0~1之间
        let original_elastic = product
            .original_elastic_distribution()
            .sample(Some((0.01, 1.0)));
        if original_price == 0.0 {
            panic!("original_price is 0.0");
        }
        // 随机生成current_range，min随机(0.0到max*0.5)，max随机(min到max*1.5)
        let mut rng = rand::thread_rng();
        let base_max = original_price * 1.5;
        // 下限范围：0.0到base_max的一半
        let min = rng.gen_range(0.0..(base_max * 0.5));
        // 上限范围：下限到base_max
        let max = rng.gen_range(min..base_max);
        let current_range = (min, max);

        Preference {
            original_price,
            original_elastic,
            current_price: original_price,
            current_range,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_new_preference() {
        let preference = Preference::new(100.0, 0.5);
        assert_eq!(preference.original_price, 100.0);
        assert_eq!(preference.original_elastic, 0.5);
        assert_eq!(preference.current_price, 0.0);
        assert_eq!(preference.current_range, (0.0, 0.0));
    }
}
