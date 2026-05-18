// Hand-authored type declarations for ZOE LA Mobile notification types.
// These types are referenced by ggen-generated push card configurations.

/** A push notification card displayed in the ZOE LA Mobile notification tray. */
export interface PushCard {
  /** Card headline shown in bold at the top of the notification tile. */
  title: string;
  /** Secondary line below the title, providing route or ministry context. */
  subtitle?: string;
  /** Main body text describing the action or update in detail. */
  body?: string;
  /** Label for the primary call-to-action button on the card. */
  action?: string;
  /** Expo deep-link URI the card action button navigates to when tapped. */
  deepLink?: string;
  /** Integer priority for ordering cards in the tray; lower values appear first. */
  priority?: number;
}
