# Monitoring and Observability

Production monitoring guide for FaniLab Smart Contracts.

## Overview

Comprehensive monitoring ensures system health and early issue detection.

## Key Metrics

### Contract Metrics
- **Invocation Count**: Total calls per contract
- **Error Rate**: Failed transactions / total
- **Gas Usage**: Average per function
- **Response Time**: Ledger confirmation time
- **Active Deliveries**: Current in-progress deliveries

### Financial Metrics
- **Total Value Locked (TVL)**: Sum of all escrows
- **Untracked Balance**: `get_untracked_balance()` per token. Represents `contract_balance - total_locked`. Indicates misclassified or accidentally transferred funds (issue #188). Should normally be zero or negligible.
- **Volume**: Total processed this period
- **Fee Revenue**: Platform fees collected
- **Average Delivery Value**: Mean escrow amount

### User Metrics
- **Active Users**: Unique addresses this period
- **New Users**: First-time users
- **Driver Count**: Active drivers
- **Completion Rate**: Delivered / Created

## Monitoring Tools

### Stellar Horizon API
Monitor transactions and contract events:
```bash
# Watch contract events
curl "https://horizon.stellar.org/accounts/$CONTRACT_ID/operations?limit=200"
```

### Custom Indexer
Build event indexer using Stellar SDK:
```javascript
const server = new StellarSdk.Server('https://horizon.stellar.org');
server.operations()
  .forAccount(contractId)
  .cursor('now')
  .stream({
    onmessage: (record) => {
      // Process events
      console.log(record);
    }
  });
```

### Dashboard (Grafana/Prometheus)
Example metrics:
- Contract invocations/hour
- Error rates
- TVL over time
- Active users

## Alerts

### Critical Alerts (Immediate Response)
- Contract error rate > 5%
- TVL drops > 20% in 1 hour
- **`untracked_balance_swept`**: protocol-level outbound transfer that can move user funds to an arbitrary recipient; treat as an immediate risk if it is observed or if `get_untracked_balance()` is non-zero.
- **`protocol_pause_status_changed`**: emergency pause toggled; any change in pause state is a site-wide operational alert that may represent incident response or governance action.
- Admin key compromise detected
- Contract balance insufficient

### High Priority (Within 1 hour)
- Error rate > 1%
- Gas usage spike > 50%
- Failed cross-contract calls
- Unusual transaction patterns
- **`funds_frozen`**: escrow entered a disputed/frozen state, indicating a live funds-protection event requiring review.
- **`escrow_holdback_marked`**: delivery was confirmed into `Holdback`; this is the intermediate state before release, dispute, or forced resolution and should be watched for lifecycle anomalies.
- **`dispute_force_resolved`**: timeout-driven resolution triggered an automatic split; this often signals a stalled or contested dispute and deserves investigation.
- **Untracked balance growth** (trending up — potential repeated fund misclassification)

### Medium Priority (Within 4 hours)
- Warning: Storage approaching limits
- Deprecated function usage
- Slow transaction confirmation

### Informational
- New version deployed
- Admin action performed
- Daily summary reports

## Event Monitoring

### Critical Events to Monitor
```rust
// Escrow events
"escrow_funded"
"escrow_released"
"escrow_refunded"
"delivery_disputed"

// Delivery events
"delivery_created"
"driver_assigned"
"delivery_confirmed"
"delivery_cancelled"

// Admin events
"protocol_initialized"
"fee_updated"
"admin_transferred"
```

> Event topics are the exact strings passed to `Symbol::new(...)` in the contracts. Rust payload types such as `ProtocolInitialized` or `FeeUpdated` are not the on-chain topic names; they are struct names used in the event payload.

### Event Processing Pipeline
1. **Capture**: Listen to Horizon stream
2. **Parse**: Extract event data
3. **Store**: Save to database
4. **Analyze**: Check for anomalies
5. **Alert**: Trigger notifications
6. **Display**: Update dashboards

## Health Checks

### Contract Health
```bash
# Check contract is responsive
stellar contract invoke \
  --id $CONTRACT_ID \
  --network mainnet \
  -- get_protocol_version
```

### Balance Health
```bash
# Verify contract has sufficient balance
stellar account balance $CONTRACT_ADDRESS --network mainnet

# Check untracked balance (should be zero or negligible)
stellar contract invoke \
  --id $ESCROW_CONTRACT_ID \
  --network mainnet \
  -- get_untracked_balance \
  --token $TOKEN_ADDRESS
```

### Escrow State Health
```bash
# Verify TVL matches expected locked amount
stellar contract invoke \
  --id $ESCROW_CONTRACT_ID \
  --network mainnet \
  -- get_total_locked \
  --token $TOKEN_ADDRESS
```

### State Health
```bash
# Sample active deliveries
stellar contract invoke \
  --id $DELIVERY_CONTRACT \
  --network mainnet \
  -- get_delivery \
  --delivery_id 12345
```

## Log Analysis

### Error Patterns
Monitor for:
- `Unauthorized` - Access control issues
- `InsufficientFunds` - Balance problems
- `InvalidState` - State machine violations
- `DeliveryNotFound` - Data integrity issues

### Usage Patterns
- Peak usage times
- Popular functions
- Average delivery lifecycle time
- Geographic distribution (off-chain)

## Performance Monitoring

### Gas Profiling
```bash
# Analyze gas usage per function
stellar contract invoke --id $CONTRACT_ID --fee-bump-account $ACCOUNT ...
```

### Bottleneck Detection
- Slow functions
- Storage hot spots
- Cross-contract call latency

## Security Monitoring

### Threat Detection
- Unusual access patterns
- Large withdrawals
- Rapid contract calls (potential exploit)
- Failed authorization attempts

### Audit Trail
- All admin actions
- All state changes
- All fund movements

## Incident Response

### Severity Levels
1. **Critical**: System down, funds at risk
2. **High**: Major feature broken
3. **Medium**: Minor feature impacted
4. **Low**: Cosmetic issue

### Response Process
1. **Detect**: Alert triggered
2. **Assess**: Determine severity
3. **Contain**: Limit damage
4. **Resolve**: Deploy fix
5. **Communicate**: Update users
6. **Post-Mortem**: Document learnings

## Dashboards

### Executive Dashboard
- TVL
- 24h Volume
- Active Users
- Error Rate

### Operations Dashboard
- Invocation counts per contract
- Error breakdown
- Gas usage trends
- Response times
- **Untracked balance** per token (should be zero or negligible)

### Financial Dashboard
- Fee revenue
- Average delivery value
- Volume by asset
- Top users by volume
- **Untracked balance trend** (detect fund misclassification issues)

## Example Alert Configuration

### Datadog/PagerDuty
```yaml
alerts:
  - name: High Error Rate
    condition: error_rate > 0.05
    severity: critical
    notify: ops-team
  
  - name: TVL Drop
    condition: tvl_change_1h < -0.2
    severity: critical
    notify: finance-team

  - name: Untracked Balance Detected
    condition: untracked_balance > 0
    severity: critical
    notify: finance-team
    runbook: "https://docs/runbooks/untracked-balance.md"
  
  - name: Gas Spike
    condition: avg_gas_30m > baseline * 1.5
    severity: high
    notify: dev-team
```

## Metrics Retention

- **Real-time**: 24 hours (1-minute granularity)
- **Short-term**: 30 days (5-minute granularity)
- **Long-term**: 2 years (1-hour granularity)
- **Archive**: Indefinite (daily summaries)

## Best Practices

1. **Alert Fatigue**: Tune thresholds to reduce noise
2. **Context**: Include runbooks with alerts
3. **Redundancy**: Multiple monitoring systems
4. **Testing**: Regularly test alert system
5. **Documentation**: Keep runbooks updated

---

**Last Updated**: January 2026
