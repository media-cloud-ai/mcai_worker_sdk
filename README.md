# Amqp Worker

[![Build Status](https://api.travis-ci.org/media-cloud-ai/rs_amqp_worker.svg?branch=master)](https://travis-ci.org/media-cloud-ai/rs_amqp_worker)
[![](http://meritbadge.herokuapp.com/amqp_worker)](https://crates.io/crates/amqp_worker)
[![Coverage Status](https://coveralls.io/repos/github/media-io/rs_amqp_worker/badge.svg?branch=master)](https://coveralls.io/github/media-io/rs_amqp_worker?branch=master)

AMQP Worker to listen and provide trait to process message.
This git repository contains library used for each worker defined in Media Cloud AI.

## Environment variables

Some variables are defined to apply a custom setting. These variables are:

| Variable name          | Default value                | Description                                 |
|------------------------|------------------------------|---------------------------------------------|
| `AMQP_HOSTNAME`        | `127.0.0.1`                  | IP or host of AMQP server                   |
| `AMQP_PORT`            | `5672`                       | AMQP server port                            |
| `AMQP_USERNAME`        | `guest`                      | User name used to connect to AMQP server    |
| `AMQP_PASSWORD`        | `guest`                      | Password used to connect to AMQP server     |
| `AMQP_VHOST`           | `/`                          | AMQP vhost                                  |
| `AMQP_TLS`             | `true`                       | Set to TRUE is HTTPS is activated.          |
| `AMQP_QUEUE`           | `job_undefined`              | AMQP queue                                  |
| `BACKEND_HOSTNAME`     | `http://127.0.0.1:4000/api`  | URL used to connect to backend server           |
| `BACKEND_USERNAME`     |                              | User name used to connect to backend server     |
| `BACKEND_PASSWORD`     |                              | Password used to connect to backend server      |




// Exchange direct_messaging (headers)
// Queue direct_messaging_(containerID/UUID)
// direct_messaging_response

broadcast=true
instance_id=
consumer_mode=file/live
job_type=
worker_name=        (manifest worker)
worker_version=
x-match=any


Payload
id requête / reference dans le header de la websocket
command:

Intance ID
urn:docker:container:
urn:docker:image:
urn:docker:network:
urn:docker:volume:
urn:uuid:0000-000


UI -> | backend -> RMQ -> Worker
      |
UI -> |
