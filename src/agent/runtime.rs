use crate::agent::input::AgentRequest;
use crate::session::SessionEvent;

pub trait AgentRuntime {
    fn run_mock_task(&self, task: &str) -> Vec<SessionEvent> {
        self.run(AgentRequest::from_task(task))
    }

    fn run(&self, request: AgentRequest) -> Vec<SessionEvent>;
}
