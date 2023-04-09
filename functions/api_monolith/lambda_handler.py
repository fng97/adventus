import random
from enum import Enum
from os import environ
from typing import Any, List, Optional

from aws_lambda_powertools import Logger
from aws_lambda_powertools.event_handler import APIGatewayRestResolver, CORSConfig
from aws_lambda_powertools.event_handler.exceptions import BadRequestError, ServiceError
from aws_lambda_powertools.logging import correlation_paths
from aws_lambda_powertools.utilities import parameters
from aws_lambda_powertools.utilities.typing import LambdaContext
from nacl.exceptions import BadSignatureError
from nacl.signing import VerifyKey
from pydantic import BaseModel, ValidationError

logger = Logger()
cors_config = CORSConfig(allow_origin="*")
app = APIGatewayRestResolver(cors=cors_config)
PUBLIC_KEY = parameters.get_parameter(
    environ["APP_PUBLIC_KEY_PARAMETER_NAME"], decrypt=True
)


class InteractionTypes(Enum):
    """Discord interaction types."""

    PING = 1
    APPLICATION_COMMAND = 2
    MESSAGE_COMPONENT = 3
    APPLICATION_COMMAND_AUTOCOMPLETE = 4
    MODAL_SUBMIT = 5


class ResponseTypes(Enum):
    """Discord interaction callback types."""

    PONG = 1
    CHANNEL_MESSAGE_WITH_SOURCE = 4


class CommandDataOptionModel(BaseModel):
    """Discord application command option object."""

    name: str
    value: Any


class InteractionDataModel(BaseModel):
    """Discord application interaction data object."""

    name: str
    options: List[CommandDataOptionModel]


class UserModel(BaseModel):
    """Discord user model: used for DMs."""

    id: str


class GuildMemberModel(BaseModel):
    """Discord guild model: used for guild messages."""

    user: UserModel


class BodyModel(BaseModel):
    """The request body model: Discord interaction object."""

    type: InteractionTypes
    data: Optional[InteractionDataModel]
    member: Optional[GuildMemberModel]
    user: Optional[UserModel]

    class Config:
        use_enum_values = True


def verify_signature(auth_sig, auth_ts, raw_body):
    """Verify the request signature. Discord will occasionally send invalid signatures
    to test our validation.

    Args:
        auth_sig (str): The signature.
        auth_ts (str): The timestamp.
        raw_body (str): The raw request body: JSON as a string.
    """
    message = auth_ts.encode() + raw_body.encode()
    verify_key = VerifyKey(bytes.fromhex(PUBLIC_KEY))
    verify_key.verify(message, bytes.fromhex(auth_sig))


@app.post("/discord")
def post_discord():
    # get necessary headers and raw body for authentication

    auth_sig = app.current_event.get_header_value(
        name="x-signature-ed25519", case_sensitive=False
    )

    auth_ts = app.current_event.get_header_value(
        name="x-signature-timestamp", case_sensitive=False
    )

    if not auth_sig or not auth_ts:
        # 400 if auth headers aren't present
        msg = "Signature verification headers missing."
        logger.info(msg)
        raise BadRequestError(msg)

    raw_body = app.current_event.body

    # verify request signature

    try:
        verify_signature(auth_sig=auth_sig, auth_ts=auth_ts, raw_body=raw_body)
    except BadSignatureError:
        msg = "Invalid request signature."
        logger.info(msg)
        raise ServiceError(401, msg)

    # parse body

    try:
        body: BodyModel = BodyModel.parse_raw(b=raw_body)
    except ValidationError as ex:
        msg = "Malformed request."
        logger.warning(msg=msg, extra={"errors": ex})
        raise BadRequestError(msg)

    # check interaction type

    if body.type == InteractionTypes.PING.value:
        return {"type": ResponseTypes.PONG.value}  # return a PONG

    if body.type == InteractionTypes.APPLICATION_COMMAND.value:
        interaction = body.data

        # check command

        if interaction.name == "roll":
            options = interaction.options

            # get user id

            if body.member:
                user_id = body.member.user.id
            else:
                user_id = body.user.id

            # roll includes number of sides and optionally number of rolls

            if len(options) == 1:  # only number of sides provided
                sides = options[0].value

                results_str = str(random.randint(1, sides))

            elif len(options) == 2:  # number of rolls also provided
                sides = options[0].value
                rolls = options[1].value

                results_str = ", ".join(
                    str(random.randint(1, sides)) for r in range(rolls)
                )

            # create message for response

            msg = f"<@{user_id}> rolled {results_str}."

            return {
                "type": ResponseTypes.CHANNEL_MESSAGE_WITH_SOURCE.value,
                "data": {
                    "tts": False,
                    "content": msg,
                    "embeds": [],
                    "allowed_mentions": {"parse": ["users"], "replied_user": True},
                },
            }

    # unfamiliar with interaction type
    return {
        "type": ResponseTypes.CHANNEL_MESSAGE_WITH_SOURCE.value,
        "data": {
            "tts": False,
            "content": "Not familiar with this command... If you receive this message, "
            "something has gone wrong.",
            "embeds": [],
            "allowed_mentions": {"parse": []},
        },
    }


@logger.inject_lambda_context(correlation_id_path=correlation_paths.API_GATEWAY_REST)
def lambda_handler(event: dict, context: LambdaContext) -> dict:
    return app.resolve(event, context)
