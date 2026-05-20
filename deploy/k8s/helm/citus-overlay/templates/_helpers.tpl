{{- define "citus-overlay.name" -}}
ai-blaise-citus
{{- end -}}

{{- define "citus-overlay.labels" -}}
app.kubernetes.io/name: {{ include "citus-overlay.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ .Chart.Name }}-{{ .Chart.Version | replace "+" "_" }}
{{- end -}}

{{- define "citus-overlay.image" -}}
{{- printf "%s/%s:%s" $.Values.global.imageRegistry .repository .tag -}}
{{- end -}}
