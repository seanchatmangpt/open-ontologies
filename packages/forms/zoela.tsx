
// Generated from ontology/zoela/*.ttl — DO NOT EDIT
// Regenerate with: ggen sync --rule zoela-forms
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';



// ============================================================================
// Household form config
// Source: ontology/zoela — households
// ============================================================================
export interface HouseholdsFormValues {
  zip: string;
  city: string;
  address_line1: string;
  household_name: string;
}

export const HouseholdsFormFields = [
  {
    name: 'zip',
    label: 'zip code',
    type: 'text',
    required: true,
    placeholder: "US postal (ZIP) code for the household's address.",
  },
  {
    name: 'city',
    label: 'city',
    type: 'text',
    required: true,
    placeholder: "City in which the household is located.",
  },
  {
    name: 'address_line1',
    label: 'address line 1',
    type: 'text',
    required: true,
    placeholder: "First line of the household's street address.",
  },
  {
    name: 'household_name',
    label: 'household name',
    type: 'text',
    required: true,
    placeholder: "Display name for the household, typically the primary family surname.",
  },
] as const;

// ============================================================================
// Connect Group form config
// Source: ontology/zoela — connect_groups
// ============================================================================
export interface ConnectGroupsFormValues {
  is_open: boolean;
  group_leader_id: string;
  current_size: number;
  max_capacity: number;
  group_frequency: string;
  group_code: string;
}

export const ConnectGroupsFormFields = [
  {
    name: 'is_open',
    label: 'Is Open',
    type: 'checkbox',
    required: true,
    placeholder: "True when the group is accepting new members (currentSize < maxCapacity and group is active).",
  },
  {
    name: 'group_leader_id',
    label: 'Group Leader ID',
    type: 'text',
    required: true,
    placeholder: "Identifier of the person holding the primary leadership role for this Connect Group.",
  },
  {
    name: 'current_size',
    label: 'Current Size',
    type: 'number',
    required: true,
    placeholder: "Current number of active members in the group. Compared against maxCapacity to compute availability.",
  },
  {
    name: 'max_capacity',
    label: 'Max Capacity',
    type: 'number',
    required: true,
    placeholder: "Maximum number of members the group can accommodate. Used to determine open/closed status.",
  },
  {
    name: 'group_frequency',
    label: 'Group Frequency',
    type: 'text',
    required: true,
    placeholder: "Recurrence cadence for group meetings. Allowed values: weekly, biweekly, monthly.",
  },
  {
    name: 'group_code',
    label: 'Group Code',
    type: 'text',
    required: true,
    placeholder: "Short alphanumeric code uniquely identifying the Connect Group within its campus (e.g. HLP-CG-01). Used as a routing key in the manufacturing pipeline.",
  },
] as const;

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
    placeholder: "Integer priority for ordering push cards in the notification tray; lower values appear first.",
  },
  {
    name: 'card_deep_link',
    label: 'card deep link',
    type: 'text',
    required: true,
    placeholder: "Expo deep-link URI the card action button navigates to when tapped.",
  },
  {
    name: 'card_action',
    label: 'card action',
    type: 'text',
    required: true,
    placeholder: "Label for the primary call-to-action button rendered on the push card (e.g. 'View Details', 'Accept Request').",
  },
  {
    name: 'card_body',
    label: 'card body',
    type: 'text',
    required: true,
    placeholder: "Main body text of the push card, describing the action or update in detail.",
  },
  {
    name: 'card_subtitle',
    label: 'card subtitle',
    type: 'text',
    required: true,
    placeholder: "Secondary line of text below the card title, providing route or ministry context.",
  },
  {
    name: 'card_title',
    label: 'card title',
    type: 'text',
    required: true,
    placeholder: "Primary headline text displayed on the push card in the notification tray.",
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
    placeholder: "Expo deep-link URI that opens the relevant screen in ZOE LA Mobile when the notification is tapped.",
  },
  {
    name: 'notification_category',
    label: 'notification category',
    type: 'text',
    required: true,
    placeholder: "Category identifier from the NotificationCategoryScheme, used to route and filter notifications in the app.",
  },
  {
    name: 'notification_body',
    label: 'notification body',
    type: 'text',
    required: true,
    placeholder: "Full body text of the push notification providing context and call-to-action.",
  },
  {
    name: 'notification_title',
    label: 'notification title',
    type: 'text',
    required: true,
    placeholder: "Short headline text of the push notification, displayed in the device notification shade.",
  },
] as const;
