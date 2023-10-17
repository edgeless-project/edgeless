#!/bin/bash

if [[ ! -d scripts || ! -d examples ]] ; then
    echo "please invoke this script from the project root"
    exit 1
fi

if [ ! -r scripts/example_template.tgz ]; then
    echo "template archive missing, bailing out"
    exit 1
fi

echo "this script will create a template workflow with a single function"
read -p "insert the name of the workflow: " workflow_name
read -p "insert the name of the function: " function_name
read -p "insert the name of the author:   " author_name

if [ -e "examples/$workflow_name" ] ; then
    echo "workflow '$workflow_name' already exists in examples/"
    exit 1
fi

function_all_lowercase=${function_name,,}
function_first_uppercase=${function_all_lowercase^}

mkdir -p examples/$workflow_name
cd examples/$workflow_name
tar xf ../../scripts/example_template.tgz
mv SMALLNAME_function ${function_all_lowercase}_function
sed -i -e "s/AUTHOR/${author_name}/g" $(rgrep AUTHOR | cut -f 1 -d : | sort -u)
sed -i -e "s/WORKFLOWNAME/${workflow_name}/g" $(rgrep WORKFLOWNAME | cut -f 1 -d : | sort -u)
sed -i -e "s/SMALLNAME/${function_all_lowercase}/g" $(rgrep SMALLNAME | cut -f 1 -d : | sort -u)
sed -i -e "s/BIGNAME/${function_first_uppercase}/g" $(rgrep BIGNAME | cut -f 1 -d : | sort -u)
cd ../../

echo "the template is ready to be used in: examples/$workflow_name"