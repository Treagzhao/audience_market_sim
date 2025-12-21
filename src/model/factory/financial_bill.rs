#[derive(Debug, Clone, Copy)]
pub struct FinancialBill {
    pub cash: f64,             //这一轮次的剩余资金
    pub sales_amount: f64,     // 这一轮的销售额
    pub total_stock: u16,      //这一轮次的总库存
    pub total_sales: u16,      //这一轮次的总销售量
    pub total_production: u16, //这一轮次的总生产量
    pub initial_stock: u16,    //这一轮次的初始库存
    pub final_stock: u16,      //这一轮次的最终库存
    pub rot_stock: u16,        //这一轮次的损失的库存
    pub remaining_stock: u16,  //这一轮次的剩余库存
}

impl FinancialBill {
    pub fn new(
        cash: f64
    ) -> Self {
        Self {
            cash,
            sales_amount: 0.0,
            total_stock: 0,
            total_sales: 0,
            total_production: 0,
            initial_stock: 0,
            final_stock: 0,
            rot_stock: 0,
            remaining_stock: 0,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_financial_bill_new() {
        let bill = FinancialBill::new(1000.0);
        assert_eq!(bill.cash, 1000.0);
        assert_eq!(bill.sales_amount, 0.0);
        assert_eq!(bill.total_stock, 0);
        assert_eq!(bill.total_sales, 0);
        assert_eq!(bill.total_production, 0);
        assert_eq!(bill.initial_stock, 0);
        assert_eq!(bill.final_stock, 0);
        assert_eq!(bill.rot_stock, 0);
        assert_eq!(bill.remaining_stock, 0);
    }
}
