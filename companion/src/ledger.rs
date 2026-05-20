// FEATURE: Sec5
// FEATURE: Sec6

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LedgerTransferPlan {
    pub transfer_id: String,
    pub debit_account: String,
    pub credit_account: String,
    pub amount_cents: i64,
    pub currency: String,
    pub previous_hash: String,
}

impl LedgerTransferPlan {
    pub fn validate(&self) -> Result<(), LedgerError> {
        validate_required("transfer_id", &self.transfer_id)?;
        validate_required("debit_account", &self.debit_account)?;
        validate_required("credit_account", &self.credit_account)?;
        validate_required("currency", &self.currency)?;
        validate_required("previous_hash", &self.previous_hash)?;
        if self.debit_account.trim() == self.credit_account.trim() {
            return Err(LedgerError::SameAccountTransfer);
        }
        if self.amount_cents <= 0 {
            return Err(LedgerError::InvalidAmount);
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<LedgerSqlPlan, LedgerError> {
        self.validate()?;
        LedgerSqlPlan::new(
            "Sec5",
            vec![format!(
                "SELECT companion_internal.ledger_transfer({}, {}, {}, {}, {}, {});",
                sql_literal(&self.transfer_id),
                sql_literal(&self.debit_account),
                sql_literal(&self.credit_account),
                self.amount_cents,
                sql_literal(&self.currency),
                sql_literal(&self.previous_hash)
            )],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LedgerSealPlan {
    pub transfer_id: String,
    pub secret_ref: String,
    pub algorithm: HmacAlgorithm,
}

impl LedgerSealPlan {
    pub fn validate(&self) -> Result<(), LedgerError> {
        validate_required("transfer_id", &self.transfer_id)?;
        validate_required("secret_ref", &self.secret_ref)
    }

    pub fn to_sql_plan(&self) -> Result<LedgerSqlPlan, LedgerError> {
        self.validate()?;
        LedgerSqlPlan::new(
            "Sec6",
            vec![format!(
                "SELECT companion_ledger_seal({}, {}, {});",
                sql_literal(&self.transfer_id),
                sql_literal(&self.secret_ref),
                sql_literal(self.algorithm.as_sql())
            )],
        )
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum HmacAlgorithm {
    Sha256,
    Sha512,
}

impl HmacAlgorithm {
    fn as_sql(self) -> &'static str {
        match self {
            Self::Sha256 => "hmac-sha256",
            Self::Sha512 => "hmac-sha512",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LedgerChainEntry {
    pub entry_hash: String,
    pub previous_hash: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LedgerChain {
    pub genesis_hash: String,
    pub entries: Vec<LedgerChainEntry>,
}

impl LedgerChain {
    pub fn validate(&self) -> Result<(), LedgerError> {
        validate_required("genesis_hash", &self.genesis_hash)?;
        let mut previous = self.genesis_hash.as_str();
        for entry in &self.entries {
            validate_required("entry_hash", &entry.entry_hash)?;
            validate_required("previous_hash", &entry.previous_hash)?;
            if entry.previous_hash != previous {
                return Err(LedgerError::BrokenHashChain);
            }
            previous = &entry.entry_hash;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LedgerSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl LedgerSqlPlan {
    fn new(feature_id: &'static str, commands: Vec<String>) -> Result<Self, LedgerError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(LedgerError::MissingRequiredField("commands"));
        }
        Ok(Self {
            feature_id,
            commands,
        })
    }

    pub fn script(&self) -> String {
        self.commands.join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LedgerError {
    BrokenHashChain,
    InvalidAmount,
    MissingRequiredField(&'static str),
    SameAccountTransfer,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BrokenHashChain => write!(formatter, "ledger hash chain is broken"),
            Self::InvalidAmount => write!(formatter, "amount_cents must be greater than zero"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::SameAccountTransfer => {
                write!(formatter, "debit_account and credit_account must differ")
            }
        }
    }
}

impl Error for LedgerError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), LedgerError> {
    if value.trim().is_empty() {
        return Err(LedgerError::MissingRequiredField(field));
    }
    Ok(())
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_transfer_renders_append_only_call() {
        let plan = LedgerTransferPlan {
            transfer_id: "tr_001".to_string(),
            debit_account: "cash".to_string(),
            credit_account: "revenue".to_string(),
            amount_cents: 5000,
            currency: "USD".to_string(),
            previous_hash: "genesis".to_string(),
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "Sec5");
        assert!(plan.script().contains("ledger_transfer"));
    }

    #[test]
    fn ledger_rejects_same_account_transfer() {
        let plan = LedgerTransferPlan {
            transfer_id: "tr_001".to_string(),
            debit_account: "cash".to_string(),
            credit_account: "cash".to_string(),
            amount_cents: 5000,
            currency: "USD".to_string(),
            previous_hash: "genesis".to_string(),
        };

        assert_eq!(plan.validate(), Err(LedgerError::SameAccountTransfer));
    }

    #[test]
    fn ledger_seal_renders_hmac_contract() {
        let plan = LedgerSealPlan {
            transfer_id: "tr_001".to_string(),
            secret_ref: "vault://ledger/hmac".to_string(),
            algorithm: HmacAlgorithm::Sha256,
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "Sec6");
        assert!(plan.script().contains("companion_ledger_seal"));
    }

    #[test]
    fn ledger_chain_detects_broken_hash_link() {
        let chain = LedgerChain {
            genesis_hash: "genesis".to_string(),
            entries: vec![LedgerChainEntry {
                entry_hash: "hash-2".to_string(),
                previous_hash: "hash-1".to_string(),
            }],
        };

        assert_eq!(chain.validate(), Err(LedgerError::BrokenHashChain));
    }
}
