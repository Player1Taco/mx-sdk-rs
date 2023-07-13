use multiversx_sc::types::H256;

use crate::{
    api::StaticApi,
    scenario::model::{AddressValue, BigUintValue, BytesValue, TxCall, TxESDT, TxExpect, U64Value},
    scenario_model::TxResponse,
};

use crate::multiversx_sc::{
    codec::{CodecFrom, PanicErrorHandler, TopEncodeMulti},
    types::{ContractCall, ManagedArgBuffer},
};

use super::TypedScCall;

#[derive(Default)]
pub struct ScCallStep {
    pub id: String,
    pub tx_id: Option<String>,
    pub explicit_tx_hash: Option<H256>,
    pub comment: Option<String>,
    pub tx: Box<TxCall>,
    pub expect: Option<TxExpect>,
    pub response: Option<TxResponse>,
    pub response_handlers: Vec<Box<dyn FnMut(&TxResponse)>>,
}

impl ScCallStep {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn response(&self) -> &TxResponse {
        self.response.as_ref().unwrap()
    }

    pub fn from<A>(mut self, address: A) -> Self
    where
        AddressValue: From<A>,
    {
        self.tx.from = AddressValue::from(address);
        self
    }

    pub fn to<A>(mut self, address: A) -> Self
    where
        AddressValue: From<A>,
    {
        self.tx.to = AddressValue::from(address);
        self
    }

    pub fn egld_value<A>(mut self, amount: A) -> Self
    where
        BigUintValue: From<A>,
    {
        if !self.tx.esdt_value.is_empty() && self.tx.egld_value.value > 0u32.into() {
            panic!("Cannot transfer both EGLD and ESDT");
        }

        self.tx.egld_value = BigUintValue::from(amount);
        self
    }

    pub fn esdt_transfer<T, N, A>(mut self, token_id: T, token_nonce: N, amount: A) -> Self
    where
        BytesValue: From<T>,
        U64Value: From<N>,
        BigUintValue: From<A>,
    {
        if self.tx.egld_value.value > 0u32.into() {
            panic!("Cannot transfer both EGLD and ESDT");
        }

        self.tx.esdt_value.push(TxESDT {
            esdt_token_identifier: BytesValue::from(token_id),
            nonce: U64Value::from(token_nonce),
            esdt_value: BigUintValue::from(amount),
        });

        self
    }

    pub fn function(mut self, expr: &str) -> Self {
        self.tx.function = expr.to_string();
        self
    }

    pub fn argument<A>(mut self, expr: A) -> Self
    where
        BytesValue: From<A>,
    {
        self.tx.arguments.push(BytesValue::from(expr));
        self
    }

    pub fn gas_limit<V>(mut self, value: V) -> Self
    where
        U64Value: From<V>,
    {
        self.tx.gas_limit = U64Value::from(value);
        self
    }

    pub fn expect(mut self, expect: TxExpect) -> Self {
        self.expect = Some(expect);
        self
    }

    /// Sets following fields based on the smart contract proxy:
    /// - "to"
    /// - "function"
    /// - "arguments"
    pub fn call<CC>(mut self, contract_call: CC) -> TypedScCall<CC::OriginalResult>
    where
        CC: ContractCall<StaticApi>,
    {
        let (to_str, function, egld_value_expr, scenario_args) =
            process_contract_call(contract_call);
        self = self.to(to_str.as_str());
        self = self.function(function.as_str());
        self = self.egld_value(egld_value_expr);
        for arg in scenario_args {
            self = self.argument(arg.as_str());
        }
        self.into()
    }

    /// Sets following fields based on the smart contract proxy:
    /// - "to"
    /// - "function"
    /// - "arguments"
    /// - "expect"
    ///     - "out"
    ///     - "status" set to 0
    pub fn call_expect<CC, ExpectedResult>(
        self,
        contract_call: CC,
        expected_value: ExpectedResult,
    ) -> TypedScCall<CC::OriginalResult>
    where
        CC: ContractCall<StaticApi>,
        ExpectedResult: CodecFrom<CC::OriginalResult> + TopEncodeMulti,
    {
        let typed = self.call(contract_call);
        typed.expect_value(expected_value)
    }

    pub fn trigger_handler(&mut self) {
        let response = self.response.clone().expect("response not yet ready");
        let mut current_handlers = std::mem::take(&mut self.response_handlers);
        for handler in current_handlers.iter_mut() {
            handler(&response);
        }
    }

    pub(crate) fn push_response_handler<F>(&mut self, f: F)
    where
        F: FnMut(&TxResponse) + 'static,
    {
        self.response_handlers.push(Box::new(f));
    }

    pub fn with_raw_response<F>(mut self, f: F) -> Self
    where
        F: FnMut(&TxResponse) + 'static,
    {
        self.push_response_handler(f);
        self
    }
}

impl AsMut<ScCallStep> for ScCallStep {
    fn as_mut(&mut self) -> &mut ScCallStep {
        self
    }
}

/// Extracts
/// - recipient,
/// - endpoint name,
/// - the arguments.
pub(super) fn process_contract_call<CC>(
    contract_call: CC,
) -> (String, String, BigUintValue, Vec<String>)
where
    CC: ContractCall<StaticApi>,
{
    let normalized_cc = contract_call.into_normalized();
    let to_str = format!(
        "0x{}",
        hex::encode(normalized_cc.basic.to.to_address().as_bytes())
    );
    let function = String::from_utf8(
        normalized_cc
            .basic
            .endpoint_name
            .to_boxed_bytes()
            .into_vec(),
    )
    .unwrap();
    let egld_value_expr = BigUintValue::from(normalized_cc.egld_payment);
    let scenario_args = convert_call_args(&normalized_cc.basic.arg_buffer);
    (to_str, function, egld_value_expr, scenario_args)
}

pub fn convert_call_args(arg_buffer: &ManagedArgBuffer<StaticApi>) -> Vec<String> {
    arg_buffer
        .to_raw_args_vec()
        .iter()
        .map(|arg| format!("0x{}", hex::encode(arg)))
        .collect()
}

pub(super) fn format_expect<T: TopEncodeMulti>(t: T) -> TxExpect {
    let mut encoded = Vec::<Vec<u8>>::new();
    let Ok(()) = t.multi_encode_or_handle_err(&mut encoded, PanicErrorHandler);
    let mut expect = TxExpect::ok().no_result();
    for encoded_res in encoded {
        let encoded_hex_string = format!("0x{}", hex::encode(encoded_res.as_slice()));
        expect = expect.result(encoded_hex_string.as_str());
    }
    expect
}

impl Clone for ScCallStep {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            tx_id: self.tx_id.clone(),
            explicit_tx_hash: self.explicit_tx_hash.clone(),
            comment: self.comment.clone(),
            tx: self.tx.clone(),
            expect: self.expect.clone(),
            response: self.response.clone(),
            response_handlers: Vec::new(),
        }
    }
}

impl std::fmt::Debug for ScCallStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScCallStep")
            .field("id", &self.id)
            .field("tx_id", &self.tx_id)
            .field("explicit_tx_hash", &self.explicit_tx_hash)
            .field("comment", &self.comment)
            .field("tx", &self.tx)
            .field("expect", &self.expect)
            .field("response", &self.response)
            .finish()
    }
}
