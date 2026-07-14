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
- Admin key compromise detected
- Contract balance insufficient

### High Priority (Within 1 hour)
- Error rate > 1%
- Gas usage spike > 50%
- Failed cross-contract calls
- Unusual transaction patterns

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
escrow_funded
escrow_released
escrow_refunded
delivery_disputed

// Delivery events
delivery_created
driver_assigned
delivery_confirmed
delivery_cancelled

// Admin events
ProtocolInitialized
FeeUpdated
AdminTransferred
```

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

### Financial Dashboard
- Fee revenue
- Average delivery value
- Volume by asset
- Top users by volume

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
