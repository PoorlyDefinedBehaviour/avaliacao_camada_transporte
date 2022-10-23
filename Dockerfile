####################################################################################################
## Builder
####################################################################################################
FROM rust:latest AS builder

RUN update-ca-certificates

# Create appuser
ENV USER=whatsapp2_user
ENV UID=10001

RUN adduser \
  --disabled-password \
  --gecos "" \
  --home "/nonexistent" \
  --shell "/sbin/nologin" \
  --no-create-home \
  --uid "${UID}" \
  "${USER}"


WORKDIR /whatsapp2

COPY ./ .

RUN cargo build --bin server --release

####################################################################################################
## Final image
####################################################################################################
FROM gcr.io/distroless/cc

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /whatsapp2

# Copy our build
COPY --from=builder /whatsapp2/target/release/server ./

# Use an unprivileged user.
USER whatsapp2_user

ENTRYPOINT ["./server"]