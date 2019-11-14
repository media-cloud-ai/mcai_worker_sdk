
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
			"identifier": "my_parameter",
			"label": "My parameter",
			"kind": ["string"],
			"required": True,
		}
	]

def process(message):
	# be able to raise, return job in errors
	# raise "my error"

	#do some stuff here

	return
