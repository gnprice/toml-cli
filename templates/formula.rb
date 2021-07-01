{% set BIN = BIN | default(value=NAME | lower) %}
{%- set HOMEPAGE = HOMEPAGE | default(value=SITE ~ "/" ~ REPO) -%}

class {{ NAME }} < Formula
  desc "{{ DESCRIPTION }}"
  homepage "{{ HOMEPAGE }}"
  url "{{ SITE }}/{{ REPO }}/releases/download/v{{ VERSION }}/{{ ARCHIVE | default(value=BIN ~"_macos_v" ~ VERSION) }}.tar.gz"
  sha256 "{{ SHA256 }}"
  version "{{ VERSION }}"

  def install
    bin.install "{{ BIN }}"
  end
end