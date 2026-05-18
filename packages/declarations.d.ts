// Type stubs for external UI packages — installed in app workspace, not root.
// This file prevents tsc errors when type-checking generated React Native files
// at root level. These stubs are intentionally minimal; the full types come from
// @types/react, @types/react-native, etc. in the app package.json.

declare module 'react' {
  export function useState<T>(init: T | (() => T)): [T, (val: T) => void];
  export function useEffect(fn: () => void | (() => void), deps?: unknown[]): void;
  export const createElement: (...args: unknown[]) => unknown;
  const React: {
    useState: typeof useState;
    useEffect: typeof useEffect;
    createElement: typeof createElement;
  };
  export default React;
}

declare module 'react-native' {
  import * as React from 'react';

  type Style = Record<string, unknown>;

  interface ViewProps { style?: Style | Style[]; children?: React.ReactNode; [key: string]: unknown }
  interface TextProps { style?: Style | Style[]; children?: React.ReactNode; [key: string]: unknown }
  interface TextInputProps {
    style?: Style | Style[];
    value?: string;
    onChangeText?: (text: string) => void;
    placeholder?: string;
    multiline?: boolean;
    [key: string]: unknown;
  }
  interface TouchableOpacityProps {
    style?: Style | Style[];
    onPress?: () => void;
    children?: React.ReactNode;
    [key: string]: unknown;
  }
  interface ScrollViewProps { style?: Style | Style[]; children?: React.ReactNode; [key: string]: unknown }
  interface ActivityIndicatorProps { [key: string]: unknown }

  export const View: React.ComponentType<ViewProps>;
  export const Text: React.ComponentType<TextProps>;
  export const TextInput: React.ComponentType<TextInputProps>;
  export const TouchableOpacity: React.ComponentType<TouchableOpacityProps>;
  export const StyleSheet: { create: <T extends object>(styles: T) => T };
  export const ScrollView: React.ComponentType<ScrollViewProps>;
  export const FlatList: React.ComponentType<Record<string, unknown>>;
  export const ActivityIndicator: React.ComponentType<ActivityIndicatorProps>;
}

declare module 'react-hook-form' {
  export function useForm<T = Record<string, unknown>>(options?: unknown): {
    register: (name: string) => unknown;
    handleSubmit: (fn: (data: T) => void) => (e?: unknown) => void;
    formState: { errors: Record<string, { message?: string }> };
    watch: (name?: string) => unknown;
    setValue: (name: string, value: unknown) => void;
    reset: (values?: Partial<T>) => void;
  };
  export type SubmitHandler<T> = (data: T) => void;
  export type UseFormReturn<T = Record<string, unknown>> = ReturnType<typeof useForm<T>>;
}

declare module '@hookform/resolvers/zod' {
  export function zodResolver(schema: unknown): unknown;
}
