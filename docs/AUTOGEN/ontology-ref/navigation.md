# navigation.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/navigation.ttl`
- **Triples:** 249
- **Classes:** 1 · **Properties:** 11 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `AppScreen` | App Screen | A navigable screen in the ZOE LA Mobile Expo app. Each screen belongs to a tab navigator stack and may be role-gated. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `isAdminOnly` | AppScreen | boolean | When true, this screen is only rendered in the tab bar for users with elevated r |
| `navRoute` | AppScreen | string | PascalCase React Navigation route name used in navigation.navigate() calls (e.g. |
| `parentTab` | AppScreen | Concept | The bottom-tab navigator concept (from zoe:AppTabScheme in categories.ttl) that  |
| `primaryObjectType` | AppScreen | string | The main domain entity type displayed or managed by this screen (e.g. 'events',  |
| `primaryRouteCategory` | AppScreen | string | The route category code from zoe:RouteCategoryScheme most relevant to this scree |
| `requiredRole` | AppScreen | string | Minimum role code a user must hold to view this screen. Values align with zoe:Ro |
| `requiresConsent` | AppScreen | boolean | When true, the screen must verify that the current user has active consent befor |
| `screenId` | AppScreen | string | Stable snake_case identifier for the screen, used as a key in navigation registr |
| `screenLabel` | AppScreen | string | Human-readable display title shown in the header bar and accessibility labels fo |
| `screenOrder` | AppScreen | integer | Integer sort order for this screen within its parent tab stack. Lower numbers ap |
| `screenStack` | AppScreen | string | Name of the React Navigation stack navigator this screen belongs to (e.g. 'HomeS |
