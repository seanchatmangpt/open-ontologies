# event.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/event.ttl`
- **Triples:** 138
- **Classes:** 2 · **Properties:** 11 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `https://zoela.org/ontology/EventRegistration` | Event Registration | A record linking a PersonProfile to a ZoeEvent, capturing sign-up time and attendance outcome. |
| `https://zoela.org/ontology/ZoeEvent` | ZOE LA Event | A church or community event organized by ZOE LA, such as a Sunday service, youth gathering, or outreach activity. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `https://zoela.org/ontology/attended` | https://zoela.org/ontology/EventRegistration | boolean | True if the registrant was confirmed as present at the event; null if attendance |
| `https://zoela.org/ontology/eventCategory` | https://zoela.org/ontology/ZoeEvent | https://zoela.org/ontology/EventCategoryScheme | Links a ZoeEvent to its EventCategory concept. |
| `https://zoela.org/ontology/eventEnd` | https://zoela.org/ontology/ZoeEvent | dateTime | ISO-8601 datetime at which the event concludes. |
| `https://zoela.org/ontology/eventId` | https://zoela.org/ontology/EventRegistration | https://zoela.org/ontology/ZoeEvent | Links an EventRegistration to the ZoeEvent being registered for. |
| `https://zoela.org/ontology/eventLocation` | https://zoela.org/ontology/ZoeEvent | string | Free-text description or address of the venue where the event takes place. |
| `https://zoela.org/ontology/eventStart` | https://zoela.org/ontology/ZoeEvent | dateTime | ISO-8601 datetime at which the event begins. |
| `https://zoela.org/ontology/eventTitle` | https://zoela.org/ontology/ZoeEvent | string | Human-readable title displayed on the event listing screen. |
| `https://zoela.org/ontology/isPublic` | https://zoela.org/ontology/ZoeEvent | boolean | True if the event is visible to unregistered visitors; false if restricted to me |
| `https://zoela.org/ontology/maxAttendees` | https://zoela.org/ontology/ZoeEvent | integer | Maximum number of registrants permitted for the event; zero or absent means unli |
| `https://zoela.org/ontology/registeredAt` | https://zoela.org/ontology/EventRegistration | dateTime | ISO-8601 datetime when the person submitted their registration. |
| `https://zoela.org/ontology/registrantId` | https://zoela.org/ontology/EventRegistration | https://zoela.org/ontology/PersonProfile | Links an EventRegistration to the PersonProfile of the person who registered. |
