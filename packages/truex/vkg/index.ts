import { OcelEvent, Receipt } from '@truex/receipt';

export interface Triple {
  subject: string;
  predicate: string;
  object: string | number | boolean;
}

export class VirtualGraph {
  private triples: Triple[] = [];

  constructor(receipt: Receipt) {
    this.project(receipt);
  }

  private project(receipt: Receipt): void {
    const events: OcelEvent[] = receipt.observed_ocel?.['ocel:events'] ?? [];

    this.addTriple('trace', 'a', 'truex:ExecutionTrace');
    this.addTriple('trace', 'truex:boundBy', receipt.receipt_hash ?? 'unknown');

    events.forEach((event, index) => {
      const subject = `event_${index}`;
      this.addTriple(subject, 'a', 'truex:Event');
      this.addTriple(subject, 'truex:activity', event['ocel:activity']);

      const valueMap = event['ocel:vmap'] ?? {};
      Object.entries(valueMap).forEach(([key, value]) => {
        if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
          this.addTriple(subject, `truex:${key}`, value);
        }
      });
    });

    if (receipt.raw_evidence?.exit_code !== undefined) {
      this.addTriple('trace', 'truex:hasExitCode', receipt.raw_evidence.exit_code);
    }
  }

  private addTriple(subject: string, predicate: string, object: string | number | boolean): void {
    this.triples.push({ subject, predicate, object });
  }

  query(predicate: string): Triple[] {
    return this.triples.filter(triple => triple.predicate === predicate);
  }

  allTriples(): Triple[] {
    return [...this.triples];
  }
}
