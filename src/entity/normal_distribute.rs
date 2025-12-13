use rand::Rng;
use rand_distr::Normal;

#[derive(Clone,Debug)]
pub struct NormalDistribution {
    mean: f64,
    id: u64,
    name: String,
    std_dev: f64,
}

impl NormalDistribution {
    pub fn new(mean: f64, id: u64, name: String, std_dev: f64) -> Self {
        NormalDistribution {
            mean,
            id,
            name,
            std_dev,
        }
    }

    pub fn mean(&self) -> f64 {
        self.mean
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn std_dev(&self) -> f64 {
        self.std_dev
    }

    pub fn random(id: u64, name: String, lower: Option<f64>, upper: Option<f64>) -> Self {
        let mut rng = rand::thread_rng();
        let lower = lower.unwrap_or(0.0);
        let upper = upper.unwrap_or(f64::MAX);

        let mean = rng.gen_range(lower..upper);
        let std_dev = rng.gen_range(0.1..upper.min(10.0));

        NormalDistribution {
            mean,
            id,
            name,
            std_dev,
        }
    }

    pub fn sample(&self, range: Option<(f64, f64)>) -> f64 {
        let mut rng = rand::thread_rng();
        let normal = Normal::new(self.mean, self.std_dev).unwrap();
        match range {
            Some((min, max)) => {
                // 重复生成样本，直到在指定范围内
                loop {
                    let sample = rng.sample(normal).max(0.0);
                    if sample >= min && sample <= max {
                        return sample;
                    }
                }
            }
            None => {
                // 没有指定范围，直接返回样本，最小值为0.0
                rng.sample(normal).max(0.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let mean = 10.0;
        let id = 1;
        let name = "test_distribution".to_string();
        let std_dev = 2.0;

        let dist = NormalDistribution::new(mean, id, name.clone(), std_dev);

        assert_eq!(dist.mean(), mean);
        assert_eq!(dist.id(), id);
        assert_eq!(dist.name(), name);
        assert_eq!(dist.std_dev(), std_dev);
    }

    #[test]
    fn test_random_default_range() {
        let id = 2;
        let name = "random_distribution".to_string();

        let dist = NormalDistribution::random(id, name.clone(), None, None);

        assert_eq!(dist.id(), id);
        assert_eq!(dist.name(), name);
        assert!(dist.mean() >= 0.0);
        assert!(dist.std_dev() >= 0.1 && dist.std_dev() < 10.0);
    }

    #[test]
    fn test_random_custom_range() {
        let id = 3;
        let name = "custom_range_distribution".to_string();
        let lower = 10.0;
        let upper = 20.0;

        let dist = NormalDistribution::random(id, name.clone(), Some(lower), Some(upper));

        assert_eq!(dist.id(), id);
        assert_eq!(dist.name(), name);
        assert!(dist.mean() >= lower && dist.mean() < upper);
        assert!(dist.std_dev() >= 0.1 && dist.std_dev() < upper.min(10.0));
    }

    #[test]
    fn test_sample() {
        let mean = 50.0;
        let id = 4;
        let name = "sample_distribution".to_string();
        let std_dev = 5.0;

        let dist = NormalDistribution::new(mean, id, name, std_dev);

        // 生成1000个样本，不指定范围
        let samples: Vec<f64> = (0..1000).map(|_| dist.sample(None)).collect();

        // 计算样本均值
        let sample_mean = samples.iter().sum::<f64>() / samples.len() as f64;

        // 计算样本标准差
        let variance = samples
            .iter()
            .map(|x| (x - sample_mean).powi(2))
            .sum::<f64>()
            / samples.len() as f64;
        let sample_std_dev = variance.sqrt();

        // 断言样本均值与预期均值的差异在可接受范围内（10%）
        assert!(
            (sample_mean - mean).abs() < mean * 0.1,
            "Sample mean {sample_mean} deviates too much from expected mean {mean}"
        );

        // 断言样本标准差与预期标准差的差异在可接受范围内（10%）
        assert!(
            (sample_std_dev - std_dev).abs() < std_dev * 0.1,
            "Sample std_dev {sample_std_dev} deviates too much from expected std_dev {std_dev}"
        );
    }

    #[test]
    fn test_sample_protection() {
        // 使用均值为0、标准差为10.0的分布，有很大概率生成负值
        let mean = 0.0;
        let id = 5;
        let name = "negative_distribution".to_string();
        let std_dev = 10.0;

        let dist = NormalDistribution::new(mean, id, name, std_dev);

        // 生成1000个样本，不指定范围
        let samples: Vec<f64> = (0..1000).map(|_| dist.sample(None)).collect();

        // 断言所有样本值都不小于0.0
        for sample in samples {
            assert!(sample >= 0.0, "Sample {sample} is less than 0.0");
        }
    }

    #[test]
    fn test_sample_with_range() {
        let mean = 50.0;
        let id = 6;
        let name = "range_distribution".to_string();
        let std_dev = 10.0;

        let dist = NormalDistribution::new(mean, id, name, std_dev);
        let min = 40.0;
        let max = 60.0;

        // 生成1000个样本，指定范围40.0~60.0
        let samples: Vec<f64> = (0..1000).map(|_| dist.sample(Some((min, max)))).collect();

        // 断言所有样本都在指定范围内
        for sample in samples {
            assert!(
                sample >= min && sample <= max,
                "Sample {sample} is not in range [{min}, {max}]"
            );
        }
    }
}
