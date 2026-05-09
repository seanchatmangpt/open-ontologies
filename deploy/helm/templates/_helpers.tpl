{{/* Common labels and name helpers */}}
{{- define "ontostar.name" -}}
ontostar
{{- end -}}

{{- define "ontostar.fullname" -}}
{{- printf "%s-%s" .Release.Name (include "ontostar.name" .) | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "ontostar.labels" -}}
app.kubernetes.io/name: {{ include "ontostar.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" }}
{{- end -}}

{{- define "ontostar.selectorLabels" -}}
app.kubernetes.io/name: {{ include "ontostar.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{- define "ontostar.secretName" -}}
{{- if .Values.secret.existingSecret -}}
{{ .Values.secret.existingSecret }}
{{- else -}}
{{ .Values.secret.name }}
{{- end -}}
{{- end -}}
