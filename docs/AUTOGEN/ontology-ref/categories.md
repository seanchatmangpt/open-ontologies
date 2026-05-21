# categories.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/categories.ttl`
- **Triples:** 143
- **Classes:** 1 · **Properties:** 2 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `CategoryRegistry` | Category Registry | Meta-class documenting all SKOS ConceptSchemes registered in the ZOE LA ontology suite. One instance per deployment. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `registryVersion` | CategoryRegistry | string | SemVer string for the registry snapshot, incremented when schemes are added or r |
| `totalSchemes` | CategoryRegistry | integer | Count of ConceptSchemes registered in this registry instance. |
