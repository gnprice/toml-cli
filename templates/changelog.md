{% if commits | length %}
# Changes since {{ since }}

{% for commit in commits %}
{%- set s = commit.message | split(pat=":") -%}
{% if s | length > 1 -%}
 - {{ commit.short_sha }}: {{ s.0 }} -{{ s.1 }} [@{{ commit.author }}]
{% else -%}
 - {{ commit.short_sha }}: {{ commit.message }} [@{{ commit.author }}]
{% endif -%}
{%- endfor %}

{% endif -%}
