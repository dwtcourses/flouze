use std::collections::HashMap;

use super::errors;
use super::model;

pub trait Repository {
    fn add_account(&mut self, account: &model::Account) -> errors::Result<()>;
    fn get_account(&self, account_id: &model::AccountId) -> errors::Result<model::Account>;
    fn delete_account(&mut self, account_id: &model::AccountId) -> errors::Result<()>;
    fn list_accounts(&self) -> errors::Result<Vec<model::Account>>;
    fn set_latest_transaction(&mut self, account_id: &model::AccountId, tx_id: &model::TransactionId) -> errors::Result<()>;

    fn add_transaction(&mut self, account_uuid: &model::AccountId, transaction: &model::Transaction) -> errors::Result<()>;
    fn get_transaction(&self, account_uuid: &model::AccountId, transaction_id: &model::TransactionId) -> errors::Result<model::Transaction>;
}

pub struct TransactionChain<'a> {
    repo: &'a Repository,
    account_id: model::AccountId,
    id: model::TransactionId,
}

impl<'a> TransactionChain<'a> {
    fn new(repo: &'a Repository, account_id: &model::AccountId, id: &model::TransactionId) -> TransactionChain<'a> {
        TransactionChain{
            repo: repo,
            account_id: account_id.to_owned(),
            id: id.to_owned()
        }
    }
}

impl<'a> Iterator for TransactionChain<'a> {
    type Item = errors::Result<model::Transaction>;

    fn next(&mut self) -> Option<errors::Result<model::Transaction>> {
        if self.id.is_empty() {
            return None;
        }

        let tx = match self.repo.get_transaction(&self.account_id, &self.id) {
            Err(e) => { return Some(Err(e.into())); },
            Ok(tx) => tx,
        };

        self.id = tx.parent.to_owned();

        Some(Ok(tx))
    }
}

pub fn get_transaction_chain<'a>(repo: &'a Repository, account: &model::Account) -> TransactionChain<'a> {
    TransactionChain::new(repo, &account.uuid, &account.latest_transaction)
}

pub fn get_balance(repo: &Repository, account: &model::Account) -> errors::Result<HashMap<model::PersonId, i64>> {
    let mut balance: HashMap<model::PersonId, i64> = account.members.iter().map(|m| (m.uuid.clone(), 0)).collect();
    let chain = get_transaction_chain(repo, account);

    for tx in chain {
        let tx = tx?;

        for p in tx.payed_by {
            balance.get_mut(&p.person).map(|b| *b += p.amount as i64);
        }

        for p in tx.payed_for {
            balance.get_mut(&p.person).map(|b| *b -= p.amount as i64);
        }
    }

    Ok(balance)
}

#[cfg(test)]
pub mod tests {
    use std::fmt::Debug;

    use super::*;

    pub fn expect_no_such_account<T: Debug>(r: errors::Result<T>) {
        match r {
            Err(errors::Error(errors::ErrorKind::NoSuchAccount, _)) => {},
            _ => { panic!("Expected NoSuchAccount error, got {:?}", r); }
        }
    }

    pub fn expect_no_such_transaction<T: Debug>(r: errors::Result<T>) {
        match r {
            Err(errors::Error(errors::ErrorKind::NoSuchTransaction, _)) => {},
            _ => { panic!("Expected NoSuchTransaction error, got {:?}", r); }
        }
    }

    pub fn make_test_account() -> model::Account {
        model::Account{
            uuid: model::generate_account_id(),
            label: "Test account".to_owned(),
            latest_transaction: vec!(),
            members: vec!(model::Person{
                uuid: model::generate_person_id(),
                name: "Member 1".to_owned(),
            }, model::Person{
                uuid: model::generate_person_id(),
                name: "Member 2".to_owned(),
            }),
        }
    }

    pub fn make_test_transaction_1(account: &model::Account) -> model::Transaction {
         model::Transaction{
            uuid: model::generate_transaction_id(),
            parent: vec!(),
            amount: 35,
            payed_by: vec!(
                model::PayedBy{
                    person: account.members[0].uuid.to_owned(),
                    amount: 35,
                }
            ),
            payed_for: vec!(
                model::PayedFor{
                    person: account.members[0].uuid.to_owned(),
                    amount: 17,
                },
                model::PayedFor{
                    person: account.members[1].uuid.to_owned(),
                    amount: 18,
                },
            ),
            label: "Fish & Chips".to_owned(),
            timestamp: 1530288593,
            deleted: false,
            replaces: vec!(),
        }
    }

    pub fn make_test_transaction_2(account: &model::Account, parent: &model::TransactionId) -> model::Transaction {
         model::Transaction{
            uuid: model::generate_transaction_id(),
            parent: parent.to_owned(),
            amount: 10,
            payed_by: vec!(
                model::PayedBy{
                    person: account.members[1].uuid.to_owned(),
                    amount: 10,
                }
            ),
            payed_for: vec!(
                model::PayedFor{
                    person: account.members[0].uuid.to_owned(),
                    amount: 10,
                },
            ),
            label: "Book".to_owned(),
            timestamp: 1530289903,
            deleted: false,
            replaces: vec!(),
        }
    }

    pub fn test_account_crud(repo: &mut Repository) {
        let account = make_test_account();

        expect_no_such_account(repo.get_account(&account.uuid));
        assert_eq!(repo.list_accounts().unwrap(), vec!());
        repo.add_account(&account).unwrap();

        let mut fetched = repo.get_account(&account.uuid).unwrap();
        assert_eq!(fetched, account);
        assert_eq!(repo.list_accounts().unwrap(), vec!(account.clone()));

        fetched.label = "New fancy name".to_owned();
        repo.add_account(&fetched).unwrap();

        let fetched2 = repo.get_account(&account.uuid).unwrap();
        assert_eq!(fetched2, fetched);

        expect_no_such_account(repo.delete_account(&model::generate_account_id()));
        repo.delete_account(&account.uuid).unwrap();
        expect_no_such_account(repo.delete_account(&account.uuid));
        expect_no_such_account(repo.get_account(&account.uuid));
    }

    pub fn test_transaction_insert(repo: &mut Repository) {
        let account = make_test_account();
        repo.add_account(&account).unwrap();

        let tx = make_test_transaction_1(&account);

        expect_no_such_transaction(repo.get_transaction(&account.uuid, &tx.uuid));
        repo.add_transaction(&account.uuid, &tx).unwrap();

        let mut fetched = repo.get_transaction(&account.uuid, &tx.uuid).unwrap();
        assert_eq!(fetched, tx);

        fetched.timestamp = 1530289104;
        repo.add_transaction(&account.uuid, &fetched).unwrap();

        let fetched2 = repo.get_transaction(&account.uuid, &tx.uuid).unwrap();
        assert_eq!(fetched2, fetched);
    }

    pub fn test_transaction_chain(repo: &mut Repository) {
        let mut account = make_test_account();
        repo.add_account(&account).unwrap();

        {
            let mut chain = get_transaction_chain(repo, &account);
            assert!(chain.next().is_none());
        }

        let tx1 = make_test_transaction_1(&account);

        {
            repo.add_transaction(&account.uuid, &tx1).unwrap();
            account.latest_transaction = tx1.uuid.to_owned();

            let mut chain = get_transaction_chain(repo, &account);
            assert_eq!(chain.next().unwrap().unwrap(), tx1);
            assert!(chain.next().is_none());
        }

        let tx2 = make_test_transaction_2(&account, &tx1.uuid);

        {
            repo.add_transaction(&account.uuid, &tx2).unwrap();
            account.latest_transaction = tx2.uuid.to_owned();

            let mut chain = get_transaction_chain(repo, &account);
            assert_eq!(chain.next().unwrap().unwrap(), tx2);
            assert_eq!(chain.next().unwrap().unwrap(), tx1);
            assert!(chain.next().is_none());
        }
    }

    pub fn test_balance(repo: &mut Repository) {
        let mut account = make_test_account();
        let tx1 = make_test_transaction_1(&account);
        let tx2 = make_test_transaction_2(&account, &tx1.uuid);

        repo.add_account(&account).unwrap();
        repo.add_transaction(&account.uuid, &tx1).unwrap();
        repo.add_transaction(&account.uuid, &tx2).unwrap();
        account.latest_transaction = tx2.uuid.to_owned();

        let balance = get_balance(repo, &account).unwrap();
        assert_eq!(balance.get(&account.members[0].uuid).unwrap(), &8);
        assert_eq!(balance.get(&account.members[1].uuid).unwrap(), &-8);
    }
}
