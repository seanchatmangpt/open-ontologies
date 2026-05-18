
// Screen: Group Interest Form
// Tab: GROUPS | Stack: GroupsStack
// Required role: MEMBER

import React from 'react';
import { ScrollView, Text, StyleSheet } from 'react-native';
import { useNavigation } from '@react-navigation/native';

export function GroupInterestFormScreen() {
  const navigation = useNavigation();
  return (
    <ScrollView style={styles.container}>
      <Text style={styles.title}>Group Interest Form</Text>
      {/* Object type: connect_groups */}
      {/* Generated from ontology/zoela/navigation.ttl */}
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: '#fff' },
  title: { fontSize: 24, fontWeight: 'bold', padding: 16 },
});
