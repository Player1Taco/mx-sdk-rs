use crate::{num_bigint::BigUint, scenario::model::ScQueryStep, scenario_model::TxResponse};
use multiversx_chain_vm::{
    tx_execution::execute_current_tx_context_input,
    tx_mock::{TxInput, TxResult},
};

use super::{check_tx_output, tx_input_util::generate_tx_hash, ScenarioVMRunner};

impl ScenarioVMRunner {
    /// Adds a SC query step, as specified in the `sc_query_step` argument, then executes it.
    pub fn perform_sc_query(&mut self, sc_query_step: &ScQueryStep) -> TxResult {
        self.perform_sc_query_lambda_and_check(sc_query_step, execute_current_tx_context_input)
    }

    pub fn perform_sc_query_update_results(&mut self, sc_query_step: &mut ScQueryStep) {
        let tx_result =
            self.perform_sc_query_lambda_and_check(sc_query_step, execute_current_tx_context_input);
        let response = TxResponse::from_tx_result(tx_result);
        sc_query_step.response = Some(response);
    }

    pub fn perform_sc_query_lambda<F>(&mut self, sc_query_step: &ScQueryStep, f: F) -> TxResult
    where
        F: FnOnce(),
    {
        let tx_input = tx_input_from_query(sc_query_step);
        let tx_result = self.blockchain_mock.vm.execute_sc_query_lambda(
            tx_input,
            &mut self.blockchain_mock.state,
            f,
        );
        assert!(
            tx_result.pending_calls.no_calls(),
            "Can't query a view function that performs an async call"
        );
        tx_result
    }

    pub fn perform_sc_query_lambda_and_check<F>(
        &mut self,
        sc_query_step: &ScQueryStep,
        f: F,
    ) -> TxResult
    where
        F: FnOnce(),
    {
        let tx_result = self.perform_sc_query_lambda(sc_query_step, f);
        if let Some(tx_expect) = &sc_query_step.expect {
            check_tx_output(&sc_query_step.id, tx_expect, &tx_result);
        }
        tx_result
    }
}

fn tx_input_from_query(sc_query_step: &ScQueryStep) -> TxInput {
    TxInput {
        from: sc_query_step.tx.to.to_vm_address(),
        to: sc_query_step.tx.to.to_vm_address(),
        egld_value: BigUint::from(0u32),
        esdt_values: Vec::new(),
        func_name: sc_query_step.tx.function.clone().into(),
        args: sc_query_step
            .tx
            .arguments
            .iter()
            .map(|scen_arg| scen_arg.value.clone())
            .collect(),
        gas_limit: u64::MAX,
        gas_price: 0u64,
        tx_hash: generate_tx_hash(&sc_query_step.id, &sc_query_step.explicit_tx_hash),
        ..Default::default()
    }
}
