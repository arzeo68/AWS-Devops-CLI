
# AWS Devops CLI

The AWS DevOps CLI is a tool designed to bring together several useful daily tools for DevOps on AWS.


## Features

Here is the list of features already implemented and planned. If you would like to propose new feature ideas or directly integrate a new feature, please refer to the "Contributing" section.

- [x]  Init a terraform repository
- [x]  Create a terraform module
- [x]  Connect to an ecs task through SSM
- [x]  Port forwarding from ECS and EC2
- [x]  Delete an S3 bucket (emptying it before)
- [x]  Create an S3 bucket and a dynamoDB table (to hold terraform state)
- [ ] Don't hesitate to suggest/make features


## Install

Steps to install the devops-cli:
```bash
$ git clone git@github.com:arzeo68/AWS-Devops-CLI.git
$ cd AWS-Devops-CLI
$ sudo ./install.sh
```

## Contributing
I welcome contributions from the community! If you'd like to contribute to this project, please follow the steps below:

Create an Issue
Before starting work on a new feature, bug fix, or enhancement, please create an issue in the repository.
Describe the issue, feature, or bug you're addressing clearly.
If applicable, provide steps to reproduce the problem or reference any related issues.

Create a New Branch
Create a new branch for your work. Use a descriptive branch name that relates to the issue, for example:
git checkout -b feature/issue-123-add-new-feature

Make Changes
Implement your changes, ensuring that your code adheres to the coding standards of the project. Make sure to write tests if needed.

Link Pull Request to the Issue
When you're ready to submit your changes, open a pull request (PR) from your branch to the main branch of the repository. In the PR description:

Once your pull request is submitted, the maintainers will review it and provide feedback if necessary. After approval, your PR will be merged.

Thank you for contributing!

## Authors

- [@arzeo68](https://github.com/arzeo68)


## 🚀 About Me
I am currently an AWS cloud engineer, my goal is to create a suite of tools that will enable developers to work with simplicity.
