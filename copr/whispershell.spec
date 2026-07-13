Name:           whispershell
Version:        0.1.0
Release:        1%{?dist}
Summary:        Secure offline voice-to-text dictation overlay for Wayland
License:        MIT
URL:            https://github.com/Muhammad-Shah-zaib/WhisperShell

Source0:        https://github.com/Muhammad-Shah-zaib/WhisperShell/releases/download/v%{version}/WhisperShell-%{version}-1.x86_64.rpm

# Use ExclusiveArch instead of BuildArch so alternate architecture build nodes
# (like aarch64) can parse and build the Source RPM package without crashing.
ExclusiveArch:  x86_64

# Disable debug package generation since we are repackaging a pre-built binary
%define debug_package %{nil}

%description
WhisperShell is an offline system-wide voice-to-text overlay built with Tauri. It utilizes local Whisper models to run entirely private dictations natively on Wayland.

%prep
# Extract the binary RPM contents into the build directory
rpm2cpio %{SOURCE0} | cpio -idmv

%build
# Nothing to compile

%install
# Copy the extracted directory structure directly to the installation root
mkdir -p %{buildroot}
cp -r usr/* %{buildroot}/

%files
%{_bindir}/whispershell
%{_datadir}/applications/WhisperShell.desktop
%{_datadir}/icons/hicolor/128x128/apps/whispershell.png
%{_datadir}/icons/hicolor/256x256@2/apps/whispershell.png
%{_datadir}/icons/hicolor/32x32/apps/whispershell.png