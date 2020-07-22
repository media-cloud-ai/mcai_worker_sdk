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
	Optional worker initialization function
	'''
	# TODO: be able to raise, return job in errors
	# raise Exception("my error")
	print("Initialise Python worker...")

def process(handle_callback, parameters):
	# TODO: be able to raise, return job in errors
	# raise Exception("my error")
	print("parameters: ", parameters)

	# do some stuff here

	# notify the progression (between 0 and 100)
	handle_callback.publish_job_progression(50)

	return {
		"destination_paths": ["/path/to/generated/file.ext"]
	}

def init_process(format_context, parameters):
	# TODO: be able to raise, return job in errors
	# raise Exception("my error")
	print("format_context: ", format_context)
	print("parameters: ", parameters)

	# raise "This is an error !!!"

	return [0]

def process_frame(job_id, stream_index, frame):
	# TODO: be able to raise, return job in errors
	# raise Exception("my error")
	data_length = 0
	for plane in range(0, len(frame.data)):
		data_length = data_length + len(frame.data[plane])

	if frame.width != 0 and frame.height != 0 :
		print(f"Job: {job_id} - Process video stream {stream_index} frame - PTS: {frame.pts}, image size: {frame.width}x{frame.height}, data length: {data_length}")
	else:
		print(f"Job: {job_id} - Process audio stream {stream_index} frame - PTS: {frame.pts}, sample_rate: {frame.sample_rate}Hz, channels: {frame.channels}, nb_samples: {frame.nb_samples}, data length: {data_length}")

	return {}

def ending_process():
	# TODO: be able to raise, return job in errors
	# raise Exception("my error")
	print("Ending Python worker process...")
