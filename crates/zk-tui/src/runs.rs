//! Proof runs as activities: one system in full, or every system in a comparison table.

use zk_core::{Control, ExampleId, ProofSystem, RunOptions};
use zk_i18n::Language;

use crate::activity::{Activity, Outbox};
use crate::views::{self, Comparison};

/// `/run <system> <example>`.
pub(crate) struct RunOne {
    pub(crate) system: &'static dyn ProofSystem,
    pub(crate) example: ExampleId,
    pub(crate) language: Language,
    pub(crate) options: RunOptions,
}

impl Activity for RunOne {
    fn subject(&self) -> String {
        self.system.meta().name.to_string()
    }

    fn run(self: Box<Self>, outbox: &Outbox, control: &Control) {
        let cell = outbox.open(views::running(self.system.meta(), self.example, self.language));
        let result = self.system.run(self.example, &self.options, control);
        outbox.replace(cell, views::report(self.system, self.example, &result, self.language));
    }
}

/// `/run all <example>`: systems one after another, the table updated after each.
pub(crate) struct RunAll {
    pub(crate) systems: &'static [&'static dyn ProofSystem],
    pub(crate) example: ExampleId,
    pub(crate) language: Language,
    pub(crate) options: RunOptions,
}

impl Activity for RunAll {
    fn subject(&self) -> String {
        self.systems.first().map(|system| system.meta().name.to_string()).unwrap_or_default()
    }

    fn run(self: Box<Self>, outbox: &Outbox, control: &Control) {
        let mut table = Comparison::new(self.systems, self.example);
        let cell = outbox.open(table.entry(self.language));
        for (index, system) in self.systems.iter().enumerate() {
            if control.is_cancelled() {
                break;
            }
            outbox.subject(system.meta().name);
            table.start(index);
            outbox.replace(cell, table.entry(self.language));
            let result = system.run(self.example, &self.options, control);
            table.finish(index, result);
            outbox.replace(cell, table.entry(self.language));
        }
        table.close();
        outbox.replace(cell, table.entry(self.language));
    }
}
