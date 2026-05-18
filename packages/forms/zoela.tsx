
// Generated from ontology/zoela/*.ttl — DO NOT EDIT
// Regenerate with: ggen sync --rule zoela-forms
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';



// ============================================================================
// Push Card Template form config
// Source: ontology/zoela — push_card_template
// ============================================================================
export interface PushCardTemplateFormValues {
  card_priority: number;
  card_deep_link: string;
  card_action: string;
  card_body: string;
  card_subtitle: string;
  card_title: string;
}

export const PushCardTemplateFormFields = [
  {
    name: 'card_priority',
    label: 'card priority',
    type: 'number',
    required: true,
    placeholder: 'Integer priority for ordering push cards in the notification tray; lower values appear first.',
  },
  {
    name: 'card_deep_link',
    label: 'card deep link',
    type: 'text',
    required: true,
    placeholder: 'Expo deep-link URI the card action button navigates to when tapped.',
  },
  {
    name: 'card_action',
    label: 'card action',
    type: 'text',
    required: true,
    placeholder: 'Label for the primary call-to-action button rendered on the push card (e.g. 'View Details', 'Accept Request').',
  },
  {
    name: 'card_body',
    label: 'card body',
    type: 'text',
    required: true,
    placeholder: 'Main body text of the push card, describing the action or update in detail.',
  },
  {
    name: 'card_subtitle',
    label: 'card subtitle',
    type: 'text',
    required: true,
    placeholder: 'Secondary line of text below the card title, providing route or ministry context.',
  },
  {
    name: 'card_title',
    label: 'card title',
    type: 'text',
    required: true,
    placeholder: 'Primary headline text displayed on the push card in the notification tray.',
  },
] as const;

// ============================================================================
// Push Notification form config
// Source: ontology/zoela — push_notification
// ============================================================================
export interface PushNotificationFormValues {
  deep_link_route: string;
  notification_category: string;
  notification_body: string;
  notification_title: string;
}

export const PushNotificationFormFields = [
  {
    name: 'deep_link_route',
    label: 'deep link route',
    type: 'text',
    required: true,
    placeholder: 'Expo deep-link URI that opens the relevant screen in ZOE LA Mobile when the notification is tapped.',
  },
  {
    name: 'notification_category',
    label: 'notification category',
    type: 'text',
    required: true,
    placeholder: 'Category identifier from the NotificationCategoryScheme, used to route and filter notifications in the app.',
  },
  {
    name: 'notification_body',
    label: 'notification body',
    type: 'text',
    required: true,
    placeholder: 'Full body text of the push notification providing context and call-to-action.',
  },
  {
    name: 'notification_title',
    label: 'notification title',
    type: 'text',
    required: true,
    placeholder: 'Short headline text of the push notification, displayed in the device notification shade.',
  },
] as const;
