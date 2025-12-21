use crate::model::factory::financial_bill::FinancialBill;
use std::collections::{HashMap, LinkedList};

pub struct Accountant {
    pub bills: HashMap<u64, FinancialBill>,
    moments: LinkedList<u64>,
}

impl Accountant {
    pub fn new(cash: f64) -> Self {
        let mut hash_map = HashMap::new();
        let mut list = LinkedList::new();
        let bill = FinancialBill::new(cash);
        hash_map.insert(0, bill);
        list.push_back(0);
        Self {
            bills: hash_map,
            moments: list,
        }
    }

    pub fn add_bill(&mut self, moment: u64, bill: FinancialBill) {
        self.bills.insert(moment, bill);
        self.moments.push_back(moment);

        if self.moments.len() > 20 {
            let oldest_moment = self.moments.pop_front().unwrap();
            self.bills.remove(&oldest_moment);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accountant_new() {
        let accountant = Accountant::new(0.1);
        assert_eq!(accountant.bills.len(), 1);
        assert_eq!(accountant.moments.len(), 1);
        let bill = accountant.bills.get(&0).unwrap();
        assert_eq!(bill.cash, 0.1);
    }

    #[test]
    fn test_accountant_add_bill() {
        let mut accountant = Accountant::new(0.1);
        let bill = FinancialBill {
            cash: 100.0,
            sales_amount: 50.0,
            total_stock: 100,
            total_sales: 50,
            total_production: 100,
            initial_stock: 100,
            final_stock: 50,
            rot_stock: 50,
            remaining_stock: 50,
        };
        accountant.add_bill(1, bill);
        assert_eq!(accountant.bills.len(), 2);
        assert_eq!(accountant.moments.len(), 2);

        let bill = accountant.bills.get(&1).unwrap();
        assert_eq!(bill.cash, 100.0);
        assert_eq!(bill.sales_amount, 50.0);
        assert_eq!(bill.total_stock, 100);
        assert_eq!(bill.total_sales, 50);
        assert_eq!(bill.total_production, 100);
        assert_eq!(bill.initial_stock, 100);
        assert_eq!(bill.final_stock, 50);
        assert_eq!(bill.rot_stock, 50);
        assert_eq!(bill.remaining_stock, 50);
    }

    #[test]
    fn test_accountant_add_bill_overflow() {
        let mut accountant = Accountant::new(0.1);
        for i in 0..21 {
            let bill = FinancialBill {
                cash: i as f64,
                sales_amount: i as f64,
                total_stock: i as u16,
                total_sales: i as u16,
                total_production: i as u16,
                initial_stock: i as u16,
                final_stock: i as u16,
                rot_stock: i as u16,
                remaining_stock: i as u16,
            };
            accountant.add_bill(i, bill);
        }
        assert_eq!(accountant.bills.len(), 20);
        assert_eq!(accountant.moments.len(), 20);
    }
}
