#[derive(Debug, Clone, Copy)]
pub struct FinancialBill {
    pub cash: f64,             //这一轮次的剩余资金
    pub units_sold: u16,     // 这一轮的销售额
    pub revenue: f64,          //这一轮次的销售额
    pub total_stock: u16,      //这一轮次的总库存
    pub total_production: u16, //这一轮次的总生产量
    pub initial_stock: u16,    //这一轮次的初始库存
    pub rot_stock: u16,        //这一轮次的损失的库存
    pub remaining_stock: u16,  //这一轮次的剩余库存
    pub production_cost: f64, //这一轮次的生产成本
    pub profit: f64, //这一轮次的利润
}

impl FinancialBill {
    pub fn new(
        cash: f64
    ) -> Self {
        Self {
            cash,
            units_sold: 0,
            revenue: 0.0,
            total_stock: 0,
            total_production: 0,
            initial_stock: 0,
            rot_stock: 0,
            remaining_stock: 0,
            production_cost: 0.0,
            profit: 0.0,
        }
    }

    pub fn set_units_sold(&mut self, units_sold: u16) {
        self.units_sold = units_sold;
    }
    pub fn set_revenue(&mut self, revenue: f64) {
        self.revenue = revenue;
    }
    pub fn set_total_stock(&mut self, total_stock: u16) {
        self.total_stock = total_stock;
    }
    pub fn set_total_production(&mut self, total_production: u16) {
        self.total_production = total_production;
    }
    pub fn set_initial_stock(&mut self, initial_stock: u16) {
        self.initial_stock = initial_stock;
    }
    pub fn set_rot_stock(&mut self, rot_stock: u16) {
        self.rot_stock = rot_stock;
    }
    pub fn set_remaining_stock(&mut self, remaining_stock: u16) {
        self.remaining_stock = remaining_stock;
    }
    pub fn set_production_cost(&mut self, production_cost: f64) {
        self.production_cost = production_cost;
    }
    pub fn set_profit(&mut self, profit: f64) {
        self.profit = profit;
    }
    pub fn set_cash(&mut self, cash: f64) {
        self.cash = cash;
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_financial_bill_new() {
        let bill = FinancialBill::new(1000.0);
        assert_eq!(bill.cash, 1000.0);
        assert_eq!(bill.units_sold, 0);
        assert_eq!(bill.total_stock, 0);
        assert_eq!(bill.total_production, 0);
        assert_eq!(bill.initial_stock, 0);
        assert_eq!(bill.rot_stock, 0);
        assert_eq!(bill.remaining_stock, 0);
    }

    #[test]
    fn test_set_units_sold() {
        let mut bill = FinancialBill::new(1000.0);
        bill.set_units_sold(10);
        assert_eq!(bill.units_sold, 10);
    }
    #[test]
    fn test_set_revenue() {
        let mut bill = FinancialBill::new(1000.0);
        bill.set_revenue(100.0);
        assert_eq!(bill.revenue, 100.0);
    }
    #[test]
    fn test_set_total_stock() {
        let mut bill = FinancialBill::new(1000.0);
        bill.set_total_stock(100);
        assert_eq!(bill.total_stock, 100);
    }
    #[test]
    fn test_set_total_production() {
        let mut bill = FinancialBill::new(1000.0);
        bill.set_total_production(100);
        assert_eq!(bill.total_production, 100);
    }
    #[test]
    fn test_set_initial_stock() {
        let mut bill = FinancialBill::new(1000.0);
        bill.set_initial_stock(100);
        assert_eq!(bill.initial_stock, 100);
    }

    #[test]
    fn test_set_rot_stock() {
        let mut bill = FinancialBill::new(1000.0);
        bill.set_rot_stock(100);
        assert_eq!(bill.rot_stock, 100);
    }
    #[test]
    fn test_set_remaining_stock() {
        let mut bill = FinancialBill::new(1000.0);
        bill.set_remaining_stock(100);
        assert_eq!(bill.remaining_stock, 100);
    }
    #[test]
    fn test_set_production_cost() {
        let mut bill = FinancialBill::new(1000.0);
        bill.set_production_cost(100.0);
        assert_eq!(bill.production_cost, 100.0);
    }
    #[test]
    fn test_set_profit() {
        let mut bill = FinancialBill::new(1000.0);
        bill.set_profit(100.0);
        assert_eq!(bill.profit, 100.0);
    }
    #[test]
    fn test_set_cash() {
        let mut bill = FinancialBill::new(1000.0);
        bill.set_cash(100.0);
        assert_eq!(bill.cash, 100.0);
    }


}
