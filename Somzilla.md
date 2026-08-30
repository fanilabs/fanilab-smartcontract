Issue1:#296 cancel_delivery cannot cancel a delivery that has no escrow

Problem Statement
delivery_contract::cancel_delivery unconditionally cross-calls the escrow
contract before updating its own state:

validate_transition(delivery.status, DeliveryStatus::Cancelled)
    .unwrap_or_else(|_| panic_with_error!(&env, FaniLabError::InvalidState));

let escrow_address: Address = /* ... */;
let _: () = env.invoke_contract(
    &escrow_address,
    &soroban_sdk::Symbol::new(&env, "refund_escrow"),
    soroban_sdk::vec![&env, sender.into_val(&env), u64::from(delivery_id).into_val(&env)],
);

delivery.status = DeliveryStatus::Cancelled;
escrow_contract::refund_escrow begins with load_escrow, which panics with
EscrowError::DeliveryNotFound when no escrow record exists. The panic propagates
and reverts the whole cancellation.

Delivery creation and escrow creation are separate calls on separate contracts —
create_delivery never funds an escrow (see issue #203) — so a delivery with no
escrow is an ordinary, reachable state.

Why It Matters
A sender who creates a delivery and then does not fund an escrow — because they
changed their mind, the funding transaction failed, or they simply never got to it
— has a delivery record they can never cancel. It remains Pending permanently,
occupying a delivery_id and appearing in the sender's and recipient's secondary
indexes indefinitely.

There is no alternative exit. cancel_delivery is the only path to
DeliveryStatus::Cancelled, and the other transitions require a driver
assignment and a funded escrow to be meaningful.

The failure is also opaque: the sender receives DeliveryNotFound from the escrow
contract when cancelling a delivery that plainly exists, which reads as a bug in
the caller rather than a missing precondition.

Proposed Solution
Make the escrow refund conditional on an escrow existing. The escrow contract
would need a non-panicking existence check — has_escrow(delivery_id) -> bool, or
get_escrow returning Option — so the delivery contract can skip the refund
when there is nothing to refund and still complete the cancellation.

Alternatively, tolerate the specific DeliveryNotFound failure from the escrow
call and proceed, though Soroban's error handling makes a positive existence check
the cleaner shape.

Preserve the existing ordering guarantee: the escrow call must still run before
the delivery's state is mutated, so a genuine refund failure cannot leave the
delivery cancelled with funds still locked.

Acceptance Criteria
 A delivery with no escrow can be cancelled
 A delivery with an escrow still triggers the refund before its state changes
 A genuine refund failure still reverts the whole cancellation
 The cancelled delivery's state and events are correct in both cases
 Authorization is unchanged — only the sender may cancel
 Regression test covers cancellation both with and without an escrow
Technical Notes
escrow_contract currently exposes no non-panicking existence check; get_escrow panics via an explicit has guard and load_escrow panics on a miss.
The #[cfg(test)] MockEscrowContract in delivery_contract/test.rs will need a matching method for whichever accessor is added.
Issue #204's note about cross-contract-call ordering applies: the escrow interaction deliberately precedes local state mutation so a failure rolls everything back.
Closed issue #95 added rollback coverage for failing escrow calls; extend that suite rather than duplicating it.
Relevant Files
contracts/delivery_contract/lib.rs — cancel_delivery
contracts/escrow_contract/lib.rs — refund_escrow, load_escrow, get_escrow
contracts/delivery_contract/test.rs — MockEscrowContract
Testing Requirements
Unit test: cancelling a delivery with no escrow succeeds and sets Cancelled
Regression test: cancelling a delivery with an escrow still refunds the sender
Regression test: a failing refund still reverts the cancellation
Authorization test: a non-sender still cannot cancel
State test: cancellation from Pending and from Active both behave correctly
Event test: delivery_cancelled emitted in both the escrow and no-escrow cases
Definition of Done
 Cancellation works without an escrow
 Refund ordering and rollback behavior preserved
 Tests above added and passing
 Formatting, clippy, and full suite clean
Complexity
Medium

Estimated Effort
4–8 hours

Dependencies
Conceptually paired with #294; each is independently solvable.



Issue2:#297 create_escrow never validates fleet_id, letting the sender choose where the driver's payout is routed

Problem Statement
create_escrow takes fleet_id: Option from the caller and stores it on the
EscrowRecord without any validation — the identifier appears exactly once in the
function, at the point it is written into the record.

At settlement, that stored value decides the payout destination:

if let (Some(fleet_addr), Some(fid)) = (fleet_management_addr, fleet_id) {
    let treasury: Address = env.invoke_contract(
        fleet_addr, &Symbol::new(env, "get_payout_address"),
        soroban_sdk::vec![env, driver.into_val(env), fid.into_val(env)]);
    payout_address = treasury;
}
fleet_management_contract keys membership as
DataKey::DriverFleet(fleet_id, driver), so a driver may be Active in any
number of fleets simultaneously. Nothing constrains which of them the sender may
name, and nothing ties the chosen fleet to the delivery.

Why It Matters
The sender — the party paying, and the party whose interests are opposite the
driver's on payout — unilaterally selects the fleet whose treasury receives the
driver's earnings.

Two concrete consequences follow. A sender can omit fleet_id for a driver
who is an active fleet member, routing the payment to the driver personally and
bypassing the fleet's arrangement entirely. Or a sender can name a fleet the
driver belongs to but which had nothing to do with this delivery, diverting the
earnings to that fleet's treasury.

Neither the driver nor the fleet consents to or can observe the choice: the
fleet_id is fixed at escrow creation, before the driver is necessarily even
assigned, and EscrowRecord.fleet_id is immutable thereafter.

This is a real authorization gap rather than a theft vector — the funds reach a
legitimate party either way — but "which legitimate party" is precisely what fleet
routing exists to determine, and it is currently the sender's unilateral call.

Proposed Solution
Validate the claimed fleet relationship at settlement rather than trusting the
stored value. get_payout_address already receives both the driver and the fleet
ID and already returns the driver's own address when membership is not Active,
so the membership check exists — what is missing is any constraint on the sender's
ability to pick a fleet, or to decline to.

The more robust direction is to stop taking fleet_id from the sender at all and
resolve the driver's fleet at settlement time from the fleet contract. That
requires a driver-to-fleet lookup the contract does not currently expose, since
membership is keyed by (fleet_id, driver) rather than by driver.

Whichever direction is chosen, the outcome should be that the driver's fleet
affiliation determines routing, not the sender's declaration.

Acceptance Criteria
 A sender cannot route a driver's payout to a fleet the driver is not active in
 A sender cannot bypass a driver's active fleet arrangement by omitting fleet_id
 Routing for a driver with no fleet membership is unchanged
 Routing for a driver in exactly one fleet is unchanged
 The behavior for a driver active in multiple fleets is defined and documented
 Regression test covers a sender naming a fleet the driver does not belong to
Technical Notes
DataKey::DriverFleet(FleetId, Address) means membership lookup requires knowing the fleet ID; a driver-to-fleets index does not exist and would need adding for the settlement-time resolution approach.
get_payout_address already returns the driver's address for Pending, Removed, and None statuses, so an invalid claim currently degrades to a direct payout rather than failing — that is the existing safety net.
Issue #217 covers a related but distinct problem: that routing is resolved at payout time from mutable fleet state. This issue is about who gets to assert the fleet in the first place.
Issue #272 proposes adding fleet_id to the batch creation path; whatever validation is agreed here should apply there too.
Relevant Files
contracts/escrow_contract/lib.rs — create_escrow, payout_driver, settle_escrow_funds
contracts/fleet_management_contract/lib.rs — get_payout_address, DataKey::DriverFleet
contracts/shared_types/lib.rs — EscrowRecord.fleet_id
Testing Requirements
Integration test: sender names a fleet the driver is not a member of → payout does not reach that treasury
Integration test: sender omits fleet_id for an active fleet driver → behavior matches the agreed policy
Regression test: driver with no fleet is paid directly
Regression test: driver in one fleet routes to that treasury
Edge case: driver active in two fleets — documented behavior asserted
Authorization test: the driver's own fleet membership governs the outcome
Definition of Done
 Fleet routing determined by the driver's membership rather than the sender's claim
 Multi-fleet behavior documented
 Tests above added and passing
 Formatting, clippy, and full suite clean
Complexity
High

Estimated Effort
1–2 days

Dependencies
Related to #217 and #272; each addresses a different aspect of fleet routing and all three are independently solvable.


