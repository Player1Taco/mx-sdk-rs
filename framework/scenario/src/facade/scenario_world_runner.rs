use crate::{
    facade::ScenarioWorld,
    scenario::{model::*, ScenarioRunner},
};

use super::scenario_world::Backend::{Debugger, VmGoBackend};

impl ScenarioWorld {
    pub fn for_each_runner_mut<F: FnMut(&mut dyn ScenarioRunner)>(&mut self, mut f: F) {
        match &mut self.backend {
            Debugger(cd_debugger) => {
                f(&mut cd_debugger.vm_runner);
                if let Some(trace) = &mut cd_debugger.trace {
                    f(trace);
                }
            },
            VmGoBackend => {
                panic!("the VM Go backend does not support step-by-step execution")
            },
        }
    }
}

impl ScenarioRunner for ScenarioWorld {
    fn run_external_steps(&mut self, step: &ExternalStepsStep) {
        self.for_each_runner_mut(|runner| runner.run_external_steps(step));
    }

    fn run_set_state_step(&mut self, step: &SetStateStep) {
        self.for_each_runner_mut(|runner| runner.run_set_state_step(step));
    }

    fn run_sc_call_step(&mut self, step: &ScCallStep) {
        self.for_each_runner_mut(|runner| runner.run_sc_call_step(step));
    }

    fn run_multi_sc_call_step(&mut self, steps: &[ScCallStep]) {
        self.for_each_runner_mut(|runner| runner.run_multi_sc_call_step(steps));
    }

    fn run_multi_sc_deploy_step(&mut self, steps: &[ScDeployStep]) {
        self.for_each_runner_mut(|runner| runner.run_multi_sc_deploy_step(steps));
    }

    fn run_sc_query_step(&mut self, step: &ScQueryStep) {
        self.for_each_runner_mut(|runner| runner.run_sc_query_step(step));
    }

    fn run_sc_deploy_step(&mut self, step: &ScDeployStep) {
        self.for_each_runner_mut(|runner| runner.run_sc_deploy_step(step));
    }

    fn run_transfer_step(&mut self, step: &TransferStep) {
        self.for_each_runner_mut(|runner| runner.run_transfer_step(step));
    }

    fn run_validator_reward_step(&mut self, step: &ValidatorRewardStep) {
        self.for_each_runner_mut(|runner| runner.run_validator_reward_step(step));
    }

    fn run_check_state_step(&mut self, step: &CheckStateStep) {
        self.for_each_runner_mut(|runner| runner.run_check_state_step(step));
    }

    fn run_dump_state_step(&mut self) {
        self.for_each_runner_mut(|runner| runner.run_dump_state_step());
    }
}
