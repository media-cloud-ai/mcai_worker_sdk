import json


def get_name():
    return "My python Worker"


def get_short_description():
    return "My python Worker"


def get_description():
    return """This is my long description
	over multilines
	"""


def get_version():
    return "0.0.3"


def get_parameters():
    return [
        {
            "identifier": "source_path",
            "label": "My parameter",
            "kind": ["string"],
            "required": True,
        },
        {
            "identifier": "destination_path",
            "label": "My array parameter",
            "kind": ["string"],
            "required": False,
        }
    ]


def init():
    '''
    Optional worker initialization function.
    '''

    print("Initialise Python worker...")


def process(handle_callback, parameters, job_id):
    '''
    Standard worker process function.
    '''
    print("Job ID: ", job_id)
    print("Parameters: ", parameters)

    # do some stuff here

    # notify the progression (between 0 and 100)
    handle_callback.publish_job_progression(50)

    return {
        "destination_paths": ["/path/to/generated/file.ext"]
    }
