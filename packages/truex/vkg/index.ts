import { Receipt } from '@truex/receipt';

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
    const ocel = receipt.observed_ocel || {};
    const events = ocel['ocel:events'] || [];
    
    // Core provenance triples
    this.addTriple('trace', 'a', 'truex:ExecutionTrace');
    this.addTriple('trace', 'truex:boundBy', receipt.receipt_hash || 'unknown');
    
    // Projecting events into triples
    events.forEach((ev: any, i: number) => {
      const s = `event_${i}`;
      this.addTriple(s, 'a', 'truex:Event');
      this.addTriple(s, 'truex:activity', ev['ocel:activity']);
      
      const vmap = ev['ocel:vmap'] || {};
      Object.keys(vmap).forEach(key => {
        this.addTriple(s, `truex:${key}`, vmap[key]);
      });
    });

    if (receipt.raw_evidence) {
      this.addTriple('trace', 'truex:hasExitCode', receipt.raw_evidence.exit_code);
    }
  }

  private addTriple(s: string, p: string, o: any): void {
    this.triples.push({ subject: s, predicate: p, object: o });
  }

  query(predicate: string): Triple[] {
    return this.triples.filter(t => t.predicate === predicate);
  }

  allTriples(): Triple[] {
    return [...this.triples];
  }
}