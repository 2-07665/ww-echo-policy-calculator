# OCR UDP Message Format

The desktop app accepts exactly one UDP message format.

## Encoding
- UTF-8 JSON text
- One UDP datagram per message

## JSON schema
```json
{
  "buffEntries": [
    {
      "buffName": "Crit_Rate",
      "buffValue": 87
    }
  ]
}
```

## Required rules
1. Top-level object must contain `buffEntries` only.
2. `buffEntries` length must be `1` to `5`.
3. Each entry must contain exactly:
   - `buffName` (string)
   - `buffValue` (integer)
4. `buffName` must be one of:
   - `Crit_Rate`
   - `Crit_Damage`
   - `Attack`
   - `Defence`
   - `HP`
   - `Attack_Flat`
   - `Defence_Flat`
   - `HP_Flat`
   - `ER`
   - `Basic_Attack_Damage`
   - `Heavy_Attack_Damage`
   - `Skill_Damage`
   - `Ult_Damage`
5. Duplicate `buffName` values are not allowed.
6. `buffValue` must match one of the app's valid values for that `buffName`.

## Example
```json
{
  "buffEntries": [
    { "buffName": "Crit_Rate", "buffValue": 87 },
    { "buffName": "Crit_Damage", "buffValue": 174 },
    { "buffName": "Attack", "buffValue": 101 },
    { "buffName": "Attack_Flat", "buffValue": 50 },
    { "buffName": "ER", "buffValue": 100 }
  ]
}
```
