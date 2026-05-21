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
{{- $digest := default "" .digest -}}
{{- if and $.Values.global.requireImageDigest (eq $digest "") -}}
{{- fail (printf "global.requireImageDigest=true requires an immutable digest for image %s" .repository) -}}
{{- end -}}
{{- if ne $digest "" -}}
{{- printf "%s/%s@%s" $.Values.global.imageRegistry .repository $digest -}}
{{- else -}}
{{- printf "%s/%s:%s" $.Values.global.imageRegistry .repository .tag -}}
{{- end -}}
{{- end -}}
