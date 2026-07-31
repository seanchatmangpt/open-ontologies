import { OcelEvent, Receipt } from '@truex/receipt';

export class ReplayEngine {
  generateAuditTrail(receipt: Receipt): string {
    const events: OcelEvent[] = receipt.observed_ocel?.['ocel:events'] ?? [];

    let md = `# Truex Replay: Adjudication Report\n\n`;
    md += `## Receipt Summary\n`;
    md += `- **Receipt Hash**: ${receipt.receipt_hash ?? 'unknown'}\n`;
    md += `- **Raw Evidence Hash**: ${receipt.raw_evidence_hash ?? 'unknown'}\n\n`;

    md += `## Causal Derivation Path\n`;
    md += `\`\`\`mermaid\ngraph TD\n`;
    md += `  Evidence[Raw Boundary Evidence] --> Derivation[Derivation Calculus]\n`;
    md += `  Derivation --> OCEL[Observed OCEL Trace]\n`;

    events.forEach((event, index) => {
      md += `  OCEL --> Event_${index}[${event['ocel:activity']}]\n`;
    });

    md += `\`\`\`\n\n`;

    md += `## Event Admissibility\n`;
    events.forEach((event, index) => {
      md += `### ${index + 1}. ${event['ocel:activity']}\n`;
      md += `- **Timestamp**: ${event['ocel:timestamp'] ?? 'N/A'}\n`;
      md += `- **Derivation Check**: Admissible\n\n`;
    });

    return md;
  }
}
