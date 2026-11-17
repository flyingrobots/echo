// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The call-site half of function preconditions: the facts at each Core
//! position, and the re-proof of every call to a precondition-bearing
//! function from them. See the parent module for the rules.
use super::{field, has_exactly, items, text, Contract, Judged, Refusal, MAX_NESTING, MAX_WORK};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;

/// A fact that dominates a Core position: a predicate known to hold, or one
/// known not to hold.
#[derive(Clone, Copy)]
enum Fact<'v> {
    Holds(&'v Value),
    Negated(&'v Value),
}

/// Re-proves every call to a function of `contracts` in `core`, and refuses a
/// precondition-bearing fold step. `contracts` holds, by callee coordinate,
/// exactly the functions whose preconditions are nonempty; with none, nothing
/// is walked and nothing is spent.
pub(crate) fn check_calls(core: &Value, contracts: &BTreeMap<String, Contract<'_>>) -> Judged<()> {
    if contracts.is_empty() {
        return Ok(());
    }
    let coordinate = text(core, "coordinate").ok_or(Refusal::Malformed)?;
    let mut walk = Walk {
        contracts,
        work: MAX_WORK,
        params: &[],
        args: &[],
    };
    if let Some(Value::Map(functions)) = field(core, "functions") {
        for (name, function) in functions {
            let Value::Text(name) = name else {
                return Err(Refusal::Malformed);
            };
            let mut facts: Vec<Fact<'_>> = contracts
                .get(&format!("{coordinate}.{name}"))
                .map_or(&[][..], |contract| contract.preconditions)
                .iter()
                .map(Fact::Holds)
                .collect();
            walk.position(
                field(function, "body").ok_or(Refusal::Malformed)?,
                &mut facts,
                0,
            )?;
        }
    }
    let Some(Value::Map(intents)) = field(core, "intents") else {
        return Err(Refusal::Malformed);
    };
    for (_, intent) in intents {
        if let Some(basis) = field(intent, "basis") {
            walk.position(basis, &mut Vec::new(), 0)?;
        }
        let constraints = items(intent, "inputConstraints")?
            .iter()
            .map(|constraint| field(constraint, "predicate").ok_or(Refusal::Malformed))
            .collect::<Judged<Vec<_>>>()?;
        for predicate in &constraints {
            walk.position(predicate, &mut Vec::new(), 0)?;
        }
        let mut facts = constraints.into_iter().map(Fact::Holds).collect();
        walk.position(
            field(intent, "body").ok_or(Refusal::Malformed)?,
            &mut facts,
            0,
        )?;
    }
    Ok(())
}

/// One module's call-site walk, and the substitution of the call being
/// re-proven: `params[N]` reads as `args[N]`.
struct Walk<'c, 'v> {
    contracts: &'c BTreeMap<String, Contract<'v>>,
    work: usize,
    params: &'v [Value],
    args: &'v [Value],
}

/// A Core value read either as written or, inside a callee's precondition,
/// with the call's arguments for its parameters.
#[derive(Clone, Copy)]
struct View<'v> {
    value: &'v Value,
    substituted: bool,
}
impl<'v> View<'v> {
    const fn plain(value: &'v Value) -> Self {
        Self {
            value,
            substituted: false,
        }
    }
    const fn inside(self, value: &'v Value) -> Self {
        Self {
            value,
            substituted: self.substituted,
        }
    }
}

impl<'v> Walk<'_, 'v> {
    fn spend(&mut self, units: usize) -> Judged<()> {
        self.work = self.work.checked_sub(units).ok_or(Refusal::Unproven)?;
        Ok(())
    }

    /// Walks one Core position under `facts`, re-proving each call it holds.
    fn position(
        &mut self,
        value: &'v Value,
        facts: &mut Vec<Fact<'v>>,
        depth: usize,
    ) -> Judged<()> {
        self.spend(1)?;
        if depth > MAX_NESTING {
            return Err(Refusal::Malformed);
        }
        let fields = match value {
            Value::Array(values) => {
                return values
                    .iter()
                    .try_for_each(|item| self.position(item, facts, depth + 1));
            }
            Value::Map(fields) => fields,
            _ => return Ok(()),
        };
        match text(value, "kind") {
            Some("if") => {
                let (Some(predicate), Some(then), Some(otherwise)) = (
                    field(value, "predicate"),
                    field(value, "then"),
                    field(value, "else"),
                ) else {
                    return Err(Refusal::Malformed);
                };
                self.position(predicate, facts, depth + 1)?;
                facts.push(Fact::Holds(predicate));
                let then = self.position(then, facts, depth + 1);
                facts.pop();
                then?;
                facts.push(Fact::Negated(predicate));
                let otherwise = self.position(otherwise, facts, depth + 1);
                facts.pop();
                return otherwise;
            }
            Some(kind @ ("all" | "any")) => {
                let values = items(value, "values")?;
                let inherited = facts.len();
                let mut judged = Ok(());
                for item in values {
                    judged = self.position(item, facts, depth + 1);
                    if judged.is_err() {
                        break;
                    }
                    facts.push(if kind == "all" {
                        Fact::Holds(item)
                    } else {
                        Fact::Negated(item)
                    });
                }
                facts.truncate(inherited);
                return judged;
            }
            Some("call") => {
                if let Some(contract) =
                    text(value, "callee").and_then(|callee| self.contracts.get(callee))
                {
                    let contract = *contract;
                    self.call(contract, items(value, "args")?, facts)?;
                }
            }
            // `fold` calls its step with no call site to prove at.
            Some("fold")
                if text(value, "step").is_some_and(|step| self.contracts.contains_key(step)) =>
            {
                return Err(Refusal::Malformed);
            }
            _ => {}
        }
        fields
            .iter()
            .try_for_each(|(_, item)| self.position(item, facts, depth + 1))
    }

    /// Requires every precondition of `contract`, instantiated with `args`,
    /// to follow from `facts`.
    fn call(
        &mut self,
        contract: Contract<'v>,
        args: &'v [Value],
        facts: &[Fact<'v>],
    ) -> Judged<()> {
        if args.len() != contract.params.len() {
            return Err(Refusal::Malformed);
        }
        (self.params, self.args) = (contract.params, args);
        let mut judged = Ok(());
        for precondition in contract.preconditions {
            let obligation = View {
                value: precondition,
                substituted: true,
            };
            match self.discharged(obligation, facts) {
                Ok(true) => {}
                Ok(false) => judged = Err(Refusal::Unproven),
                Err(refusal) => judged = Err(refusal),
            }
            if judged.is_err() {
                break;
            }
        }
        (self.params, self.args) = (&[], &[]);
        judged
    }

    /// The value a view reads: an argument where a substituted parameter is.
    fn resolve(&self, view: View<'v>) -> View<'v> {
        if view.substituted
            && text(view.value, "kind") == Some("local")
            && has_exactly(view.value, &["kind", "ref"])
        {
            if let Some(reference) = field(view.value, "ref") {
                if let Some(index) = self.params.iter().position(|param| param == reference) {
                    if let Some(argument) = self.args.get(index) {
                        return View::plain(argument);
                    }
                }
            }
        }
        view
    }

    /// Structural equality of two views, independent of map entry order.
    fn equal(&mut self, left: View<'v>, right: View<'v>) -> Judged<bool> {
        let (left, right) = (self.resolve(left), self.resolve(right));
        self.spend(1)?;
        match (left.value, right.value) {
            (Value::Map(a), Value::Map(b)) => {
                if a.len() != b.len() {
                    return Ok(false);
                }
                for (key, value) in a {
                    let Some((_, other)) = b.iter().find(|(other, _)| other == key) else {
                        return Ok(false);
                    };
                    if !self.equal(left.inside(value), right.inside(other))? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            (Value::Array(a), Value::Array(b)) => {
                if a.len() != b.len() {
                    return Ok(false);
                }
                for (value, other) in a.iter().zip(b) {
                    if !self.equal(left.inside(value), right.inside(other))? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            (a, b) => Ok(a == b),
        }
    }

    /// Whether the instantiated `obligation` follows from `facts`.
    fn discharged(&mut self, obligation: View<'v>, facts: &[Fact<'v>]) -> Judged<bool> {
        let value = obligation.value;
        match (text(value, "kind"), text(value, "op")) {
            (Some("true"), _) => Ok(true),
            (Some("all"), _) => {
                for item in items(value, "values")? {
                    if !self.discharged(obligation.inside(item), facts)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            (Some("compare"), Some(op @ ("<=" | ">="))) => {
                let (Some(left), Some(right)) = (field(value, "left"), field(value, "right"))
                else {
                    return Err(Refusal::Malformed);
                };
                let (left, right) = (obligation.inside(left), obligation.inside(right));
                if op == "<=" {
                    self.ordered(right, left, facts)
                } else {
                    self.ordered(left, right, facts)
                }
            }
            _ => {
                for fact in facts {
                    if self.contains(*fact, obligation)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    /// The order proof of unsigned subtraction and `slice`: `big >= small`.
    fn ordered(&mut self, big: View<'v>, small: View<'v>, facts: &[Fact<'v>]) -> Judged<bool> {
        if self.equal(big, small)? {
            return Ok(true);
        }
        let (high, low) = (self.literal(big), self.literal(small));
        if low == Some(0) || matches!((high, low), (Some(a), Some(b)) if a >= b) {
            return Ok(true);
        }
        for fact in facts {
            self.spend(1)?;
            if let Fact::Holds(fact) = fact {
                if self.orders(fact, big, small)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Whether the fact, or a conjunct of it at any depth, orders `big >= small`.
    fn orders(&mut self, fact: &'v Value, big: View<'v>, small: View<'v>) -> Judged<bool> {
        self.spend(1)?;
        match (text(fact, "kind"), text(fact, "op")) {
            (Some("all"), _) => {
                for item in items(fact, "values")? {
                    if self.orders(item, big, small)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            (Some("compare"), Some(op)) => {
                let (Some(left), Some(right)) = (field(fact, "left"), field(fact, "right")) else {
                    return Err(Refusal::Malformed);
                };
                let (left, right) = (View::plain(left), View::plain(right));
                Ok(match op {
                    "<" | "<=" => self.equal(left, small)? && self.equal(right, big)?,
                    ">" | ">=" => self.equal(left, big)? && self.equal(right, small)?,
                    "==" => {
                        (self.equal(left, big)? && self.equal(right, small)?)
                            || (self.equal(left, small)? && self.equal(right, big)?)
                    }
                    _ => false,
                })
            }
            _ => Ok(false),
        }
    }

    /// Whether `fact` proves the non-order `wanted`: it is the same predicate,
    /// the same comparison mirrored, or, for a holding `all`, a conjunct of it
    /// does at any depth. A negated fact proves only the identical negation.
    fn contains(&mut self, fact: Fact<'v>, wanted: View<'v>) -> Judged<bool> {
        self.spend(1)?;
        match fact {
            Fact::Negated(fact) => {
                let negation = text(wanted.value, "kind") == Some("not")
                    && has_exactly(wanted.value, &["kind", "value"]);
                match field(wanted.value, "value") {
                    Some(inner) if negation => self.equal(View::plain(fact), wanted.inside(inner)),
                    _ => Ok(false),
                }
            }
            Fact::Holds(fact) => {
                if self.equal(View::plain(fact), wanted)? || self.mirrors(fact, wanted)? {
                    return Ok(true);
                }
                if text(fact, "kind") == Some("all") {
                    for item in items(fact, "values")? {
                        if self.contains(Fact::Holds(item), wanted)? {
                            return Ok(true);
                        }
                    }
                }
                Ok(false)
            }
        }
    }

    /// Whether `wanted` is the comparison `fact` with its operands swapped and
    /// its operator mirrored (`a < b` and `b > a`; `a == b` and `b == a`).
    fn mirrors(&mut self, fact: &'v Value, wanted: View<'v>) -> Judged<bool> {
        let compare = |value: &Value| {
            text(value, "kind") == Some("compare")
                && has_exactly(value, &["kind", "op", "left", "right"])
        };
        if !compare(fact) || !compare(wanted.value) {
            return Ok(false);
        }
        let mirrored = match text(fact, "op") {
            Some("<") => ">",
            Some(">") => "<",
            Some("<=") => ">=",
            Some(">=") => "<=",
            Some("==") => "==",
            Some("!=") => "!=",
            _ => return Ok(false),
        };
        let (Some(left), Some(right), Some(wanted_left), Some(wanted_right)) = (
            field(fact, "left"),
            field(fact, "right"),
            field(wanted.value, "left"),
            field(wanted.value, "right"),
        ) else {
            return Ok(false);
        };
        Ok(text(wanted.value, "op") == Some(mirrored)
            && self.equal(View::plain(left), wanted.inside(wanted_right))?
            && self.equal(View::plain(right), wanted.inside(wanted_left))?)
    }

    /// A `U32` or `U64` integer literal, read through a substitution.
    fn literal(&self, view: View<'v>) -> Option<u64> {
        let value = self.resolve(view).value;
        if text(value, "kind") != Some("const") {
            return None;
        }
        let literal = field(value, "value")?;
        if text(literal, "kind") != Some("int") {
            return None;
        }
        let Some(Value::Integer(number)) = field(literal, "value") else {
            return None;
        };
        match text(literal, "width")? {
            "U32" => u32::try_from(*number).ok().map(u64::from),
            "U64" => u64::try_from(*number).ok(),
            _ => None,
        }
    }
}
