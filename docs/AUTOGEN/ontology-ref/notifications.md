# notifications.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/notifications.ttl`
- **Triples:** 229
- **Classes:** 2 · **Properties:** 13 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `PushCardTemplate` | Push Card Template | A reusable template describing the visual layout and content of a notification card rendered in the ZOE LA Mobile notifi |
| `PushNotification` | Push Notification | A push notification dispatched to a PersonProfile's device when a RouteStage transition occurs. Carries a title, body, c |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `cardAction` | PushCardTemplate | string | Label for the primary call-to-action button rendered on the push card (e.g. 'Vie |
| `cardBody` | PushCardTemplate | string | Main body text of the push card, describing the action or update in detail. |
| `cardDeepLink` | PushCardTemplate | anyURI | Expo deep-link URI the card action button navigates to when tapped. |
| `cardForRoute` | PushCardTemplate | ServiceRoute | Associates a PushCardTemplate with the ServiceRoute that triggers its rendering. |
| `cardPriority` | PushCardTemplate | integer | Integer priority for ordering push cards in the notification tray; lower values  |
| `cardSubtitle` | PushCardTemplate | string | Secondary line of text below the card title, providing route or ministry context |
| `cardTitle` | PushCardTemplate | string | Primary headline text displayed on the push card in the notification tray. |
| `deepLinkRoute` | PushNotification | anyURI | Expo deep-link URI that opens the relevant screen in ZOE LA Mobile when the noti |
| `notificationBody` | PushNotification | string | Full body text of the push notification providing context and call-to-action. |
| `notificationCategory` | PushNotification | string | Category identifier from the NotificationCategoryScheme, used to route and filte |
| `notificationTitle` | PushNotification | string | Short headline text of the push notification, displayed in the device notificati |
| `sentTo` | PushNotification | PersonProfile | Links a PushNotification to the PersonProfile whose device receives the notifica |
| `triggeredBy` | PushNotification | RouteStage | Links a PushNotification to the RouteStage whose completion or activation trigge |
