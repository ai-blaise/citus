#!/bin/bash

set -ex

# Function to get the OS version
get_rpm_os_version() {
    if [[ -f /etc/centos-release ]]; then
        cat /etc/centos-release | awk '{print $4}'
    elif [[ -f /etc/oracle-release ]]; then
        cat /etc/oracle-release | awk '{print $5}'
    else
        echo "Unknown"
    fi
}

package_type=${1}

# Since $HOME is set in GH_Actions as /github/home, pyenv fails to create virtualenvs.
# For this script, we set $HOME to /root and then set it back to /github/home.
GITHUB_HOME="${HOME}"
export HOME="/root"
REPO_ROOT="$(pwd)"
TOOLS_CHECKOUT_PARENT="$(mktemp -d)"

cleanup() {
    rm -rf "${TOOLS_CHECKOUT_PARENT}"
    export HOME="${GITHUB_HOME}"
}
trap cleanup EXIT

eval "$(pyenv init -)"
pyenv versions
pyenv virtualenv ${PACKAGING_PYTHON_VERSION} packaging_env
pyenv activate packaging_env

git clone -b v0.8.27 --depth=1 https://github.com/citusdata/tools.git "${TOOLS_CHECKOUT_PARENT}/tools"
python3 -m pip install -r "${TOOLS_CHECKOUT_PARENT}/tools/packaging_automation/requirements.txt"


echo "Package type: ${package_type}"
echo "OS version: $(get_rpm_os_version)"

 # For RHEL 7, we need to install urllib3<2 due to below execution error
 # ImportError: urllib3 v2.0 only supports OpenSSL 1.1.1+, currently the 'ssl'
 # module is compiled with 'OpenSSL 1.0.2k-fips  26 Jan 2017'.
 # See: https://github.com/urllib3/urllib3/issues/2168
if [[ ${package_type} == "rpm" && $(get_rpm_os_version) == 7* ]]; then
    python3 -m pip uninstall -y urllib3
    python3 -m pip install 'urllib3<2'
fi

(
    cd "${TOOLS_CHECKOUT_PARENT}"
    python3 -m tools.packaging_automation.validate_build_output \
        --output_file "${REPO_ROOT}/output.log" \
        --ignore_file "${REPO_ROOT}/.github/packaging/packaging_ignore.yml" \
        --package_type ${package_type}
)
pyenv deactivate
# Set $HOME back to /github/home
export HOME=${GITHUB_HOME}

# Print the output to the console
