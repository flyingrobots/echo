// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#[derive(Debug, Clone, PartialEq)]
pub struct Counter {
    pub id: String,
    pub value: i64,
}

pub struct CounterStore {
    pub counter: Counter,
}

impl ReadCounter for CounterStore {
    fn read_counter(&self) -> &Counter {
        &self.counter
    }
}

impl WriteCounter for CounterStore {
    fn write_counter(&mut self, value: Counter) {
        self.counter = value;
    }
}

pub struct Increment;

impl IncrementCounterRewrite for Increment {
    type Error = ();

    fn apply<C>(&self, ctx: &mut C, _args: IncrementCounterArgs) -> Result<Counter, Self::Error>
    where
        C: IncrementCounterContext,
    {
        ctx.delete_counter();
        Ok(ctx.read_counter().clone())
    }
}
