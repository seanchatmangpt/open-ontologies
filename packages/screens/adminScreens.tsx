
// Admin screen: Admin: Follow-Ups Overdue
// Object type: follow_up_tasks
// Route category: 
import React from 'react';
import { View, Text, FlatList, StyleSheet } from 'react-native';

export function AdminFollowUpsOverdueScreen() {
  return (
    <View style={styles.container}>
      <Text style={styles.title}>Admin: Follow-Ups Overdue</Text>
      {/* Filtered view of: follow_up_tasks */}
      {/* Required role: CARE */}
    </View>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, padding: 16 },
  title: { fontSize: 20, fontWeight: 'bold', marginBottom: 12 },
});
